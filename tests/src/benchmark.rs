use crate::TestHarness;

/// Measure CPU cycles consumed running from current PC to target address.
/// Returns (cycles, reached_target).
pub fn measure_cycles_to(
    harness: &mut TestHarness,
    target_pc: u16,
    max_cycles: u64,
) -> (u64, bool) {
    let mut cycles = 0u64;
    loop {
        if harness.pc() == target_pc {
            return (cycles, true);
        }
        if cycles >= max_cycles {
            return (cycles, false);
        }
        cycles += harness.clock() as u64;
    }
}

/// Measure cycles for a closure that operates on the harness.
/// Uses the harness `total_cycles` counter to compute the delta.
pub fn measure_cycles<F>(harness: &mut TestHarness, f: F) -> u64
where
    F: FnOnce(&mut TestHarness),
{
    let before = harness.total_cycles();
    f(harness);
    harness.total_cycles() - before
}
