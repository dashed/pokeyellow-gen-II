//! Link cable bridge for dual-instance serial communication.
//!
//! Provides a [`LinkEndpoint`] pair that implements boytacean's [`SerialDevice`]
//! trait, allowing two `GameBoy` instances to exchange bytes as if connected
//! by a Game Boy link cable.
//!
//! # Usage
//!
//! ```ignore
//! let (endpoint_a, endpoint_b) = LinkEndpoint::new_pair();
//! gb_a.attach_serial(endpoint_a);
//! gb_b.attach_serial(endpoint_b);
//!
//! // Interleave execution — both GBs advance together
//! for _ in 0..frames {
//!     gb_a.next_frame();
//!     gb_b.next_frame();
//! }
//! ```

use boytacean::serial::SerialDevice;
use std::sync::{Arc, Mutex};

/// Shared state between two sides of a link cable connection.
struct SharedLink {
    /// Byte offered by side A (read by side B via `send()`).
    a_byte: u8,
    /// Byte offered by side B (read by side A via `send()`).
    b_byte: u8,
}

/// One endpoint of a link cable bridge.
///
/// Created in pairs via [`LinkEndpoint::new_pair()`]. Each endpoint is
/// attached to a separate `GameBoy` instance. When one side initiates a
/// serial transfer, the emulator calls `send()` to get the incoming byte
/// and `receive()` to deliver the outgoing byte — both routed through
/// shared state to the other endpoint.
pub struct LinkEndpoint {
    shared: Arc<Mutex<SharedLink>>,
    is_side_a: bool,
}

impl LinkEndpoint {
    /// Create a linked pair of endpoints.
    ///
    /// Returns `(side_a, side_b)` as boxed trait objects ready for
    /// [`GameBoy::attach_serial()`].
    pub fn new_pair() -> (Box<LinkEndpoint>, Box<LinkEndpoint>) {
        let shared = Arc::new(Mutex::new(SharedLink {
            a_byte: 0xFF,
            b_byte: 0xFF,
        }));
        let side_a = Box::new(LinkEndpoint {
            shared: shared.clone(),
            is_side_a: true,
        });
        let side_b = Box::new(LinkEndpoint {
            shared,
            is_side_a: false,
        });
        (side_a, side_b)
    }
}

impl SerialDevice for LinkEndpoint {
    fn send(&mut self) -> u8 {
        let link = self.shared.lock().unwrap();
        // Return the byte offered by the OTHER side
        if self.is_side_a {
            link.b_byte
        } else {
            link.a_byte
        }
    }

    fn receive(&mut self, byte: u8) {
        let mut link = self.shared.lock().unwrap();
        // Store the byte we're sending for the OTHER side to read
        if self.is_side_a {
            link.a_byte = byte;
        } else {
            link.b_byte = byte;
        }
    }

    fn allow_slave(&self) -> bool {
        // Both sides accept external clock so either can initiate transfers
        true
    }

    fn description(&self) -> String {
        format!("LinkEndpoint({})", if self.is_side_a { "A" } else { "B" })
    }

    fn state(&self) -> String {
        let link = self.shared.lock().unwrap();
        format!("a={:02X} b={:02X}", link.a_byte, link.b_byte)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_exchanges_bytes() {
        let (mut a, mut b) = LinkEndpoint::new_pair();

        // Initially both sides return 0xFF (no data)
        assert_eq!(a.send(), 0xFF);
        assert_eq!(b.send(), 0xFF);

        // Side A sends 0x42
        a.receive(0x42);
        // Side B should now read 0x42
        assert_eq!(b.send(), 0x42);
        // Side A still reads 0xFF (B hasn't sent anything)
        assert_eq!(a.send(), 0xFF);

        // Side B sends 0xAB
        b.receive(0xAB);
        // Side A should now read 0xAB
        assert_eq!(a.send(), 0xAB);
    }

    #[test]
    fn both_allow_slave() {
        let (a, b) = LinkEndpoint::new_pair();
        assert!(a.allow_slave());
        assert!(b.allow_slave());
    }
}
