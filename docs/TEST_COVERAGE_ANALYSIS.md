# Test Coverage Analysis — Pokemon Yellow ROM Hack

Deep investigation into test coverage gaps, infrastructure capabilities, and recommendations for the pokeyellow-gen-II project.

**Date**: 2026-05-23
**Scope**: All 15 branches (14 feature + dashed/docs), 134 test files, 958 test functions

---

## Executive Summary

The test suite achieves **100% fix-level coverage** — every one of the 117 bug fixes has a dedicated test file. However, **73% of tests are ROM byte validation** (confirming patch bytes exist) rather than behavioral tests that execute game code. The result is a coverage paradox: broad but shallow.

**Key findings:**

| Metric | Value | Assessment |
|--------|-------|------------|
| Bug fixes with tests | 117/117 (100%) | Excellent breadth |
| Tests that execute game code | 247/958 (26%) | Low behavioral depth |
| ROM-byte-only tests | 696/958 (73%) | Structural, not behavioral |
| Golden image tests | 2 (0.2%) | Massively underutilized |
| E2E gameplay tests | 10 (1.0%) | Minimal scenario coverage |
| Overworld behavioral tests | 0/241 | Largest gap |
| Move effect files tested | 2/14 (14%) | Critical gap |
| Damage pipeline tested | 0% behavioral | Highest-risk gap |
| Built but unused infrastructure | 3 modules | InputScript DSL, LinkEndpoint, Debug utils |

**Bottom line**: The test suite excels at confirming *that patches were applied* but largely doesn't verify *that patches work correctly at runtime*. The damage calculation pipeline — the most critical arithmetic in the game — has zero behavioral coverage.

---

## 1. Bug Fix Coverage Audit

### Result: 117/117 Fixes Covered (100%)

Every bug fix commit across all 11 fix-bearing branches has a dedicated test file with 4-18 test functions. Coverage is genuine — tests directly verify the specific fix behavior, not just adjacent functionality.

| Branch | Fix Commits | Test Files | Test Functions | Coverage |
|--------|------------|------------|----------------|----------|
| `dashed/accuracy-crit` | 2 | 2 | 22 | 100% |
| `dashed/battle-bugs` | 44 | 40 | 298 | 100% |
| `dashed/ghost-battle` | 1 | 1 | 14 | 100% |
| `dashed/item-fixes` | 9 | 9 | 75 | 100% |
| `dashed/overworld-fixes` | 42 | 33 | 241 | 100% |
| `dashed/glitch-safety` | 9 | 6 | 39 | 100% |
| `dashed/surge-trash` | 1 | 1 | 7 | 100% |
| `dashed/transform-swap` | 1 | 1 | 12 | 100% |
| `dashed/item-overflow` | 1 | 1 | 9 | 100% |
| `dashed/qol` | 1 | 1 | 5 | 100% |
| `dashed/cosmetic` | 6 | 6 | 38 | 100% |
| **TOTAL** | **117** | **101** | **760** | **100%** |

Related fixes share test files efficiently (e.g., 3 Counter fixes share `counter.rs` with 18 tests).

---

## 2. Test Type Distribution

```
ROM Byte Validation:  ████████████████████████████████████████  98 files (73%)  696 tests (73%)
Emulator State Test:  ████████████               30 files (22%)  247 tests (26%)
E2E Gameplay Test:    ██                          3 files  (2%)   10 tests  (1%)
Golden Image Test:    █                           2 files  (1%)    2 tests  (0.2%)
Input Script Test:    ▏                           1 file   (1%)    2 tests  (0.2%)
```

### What each type verifies

- **ROM Byte**: Confirms the right opcodes/operands were patched at the right addresses. Fast, deterministic, but cannot verify runtime behavior.
- **Emulator State**: Runs actual Game Boy code with controlled WRAM state and verifies results. Tests individual functions in isolation via register injection + breakpoint pattern.
- **Golden Image**: Screenshot comparison for visual correctness.
- **E2E Gameplay**: Full game boot → input sequences → state verification. Tests entire pipeline.

---

## 3. Game System Coverage Map

### Coverage Matrix: Game System × Test Type

| Game System | ROM Byte | Emulator | Golden | E2E | Total Tests |
|---|---|---|---|---|---|
| **Battle Engine** | 159 | 183 | 0 | 0 | **342** |
| **Overworld** | 241 | **0** | 0 | 0 | **241** |
| **Items** | 40 | 35 | 0 | 0 | **75** |
| **Visual/Cosmetic** | 60 | 5 | 1 | 0 | **66** |
| **Pokémon Data** | 58 | 2 | 0 | 0 | **60** |
| **Slot Machine** | 32 | 7 | 0 | 0 | **39** |
| **Glitch Safety** | 39 | **0** | 0 | 0 | **39** |
| **Audio** | 34 | 0 | 0 | 0 | **36** |
| **Save System** | 25 | 5 | 0 | 0 | **30** |
| **E2E/Integration** | 0 | 6 | 0 | 10 | **16** |
| **Menus/QoL** | 8 | 4 | 1 | 0 | **13** |

### Critical gaps (zero behavioral tests)

1. **Overworld** (241 tests, ALL ROM byte) — 33 files verify patch bytes but no test runs NPC movement, walks through a warp, triggers a script, or tests collision.
2. **Glitch Safety** (39 tests, ALL ROM byte) — No test actually triggers a MissingNo. encounter, attempts Super Glitch, or tries ZZAZZ to confirm guards activate at runtime.
3. **Audio** (34 ROM byte tests) — Only `audio.rs` (2 tests) checks that *some* audio plays. No test verifies specific audio fixes (pitch slide, cry, music) at runtime.

### Game systems with zero dedicated tests

- Link Battle/Trade protocol
- PC Box system (deposit, withdraw, release, box switching)
- Pokémon Center / Healing flow
- Pokédex system
- Text engine
- Start Menu
- Naming Screen
- Fishing

---

## 4. High-Risk Untested Code Paths

### Tier 1: CRITICAL — Complex functions with zero behavioral coverage

| Function | File | Risk | Why |
|---|---|---|---|
| **`CalculateDamage`** | `core.asm:4655` | **CRITICAL** | 168 lines, 27 branches. Core damage formula with 16-bit overflow capping. Used in EVERY battle. Only division-by-zero guard is ROM-byte tested. |
| **`TryRunningFromBattle`** | `core.asm:1635` | **CRITICAL** | 119 lines, 15 branches. Complex arithmetic (`playerSpeed*32 / (enemySpeed/4%256) + 30*runAttempts`), overflow-prone, completely untested. |
| **`CheckForDisobedience`** | `core.asm:4176` | **CRITICAL** | 179 lines, 36 branches. 2nd most branchy function in core.asm. Badge-based obedience thresholds, random outcome selection. Completely untested. |
| **`RandomizeDamage`** | `core.asm:5893` | **CRITICAL** | Applies 85-100% damage roll via rejection sampling. Multiplies damage, divides by 255. Zero test coverage. |
| **`CalcHitChance`** | `core.asm:5821` | **CRITICAL** | Combines accuracy + evasion stages via multiplication/division. Foundation of all hit/miss. Untested despite MoveHitTest being tested. |
| **`SelectEnemyMove`** | `core.asm:3231` | **HIGH** | 93 lines, 18 branches. Move selection with Metronome/Mirror Move handling. |
| **`GainExperience`** | `experience.asm` | **HIGH** | ~200 lines. Full exp distribution, level-ups, move learning. Never executed in tests. |

### Move Effects: 12 of 14 implementations completely untested

Only `substitute.asm` and `heal.asm` have test coverage:

| Untested File | Risk | Key Concern |
|---|---|---|
| **`transform.asm`** | **P0** | Has 2 documented unfixed bugs (INVULNERABLE check reads wrong register) |
| **`drain_hp.asm`** | P1 | HP recovery calculation, substitute interaction |
| **`one_hit_ko.asm`** | P1 | Speed/level comparison for OHKO moves |
| **`recoil.asm`** | P1 | Recoil = damage/4. What if damage < 4? |
| **`haze.asm`** | P1 | Resets ALL stat stages + volatile statuses |
| **`reflect_light_screen.asm`** | P2 | Defense/special doubling flags |
| **`leech_seed.asm`** | P2 | Grass immunity, HP drain |
| **`paralyze.asm`** | P2 | Speed quartering, Electric immunity |
| **`pay_day.asm`** | P3 | BCD coin addition overflow |
| **`conversion.asm`** | P3 | Type change |
| **`focus_energy.asm`** | P3 | Crit rate flag |
| **`mist.asm`** | P3 | Stat reduction prevention flag |

### Trainer AI: 24 of 30 functions untested

All gym leader AIs (Brock through Giovanni), Elite Four AIs (Lorelei through Lance), and `AIEnemyTrainerChooseMoves` (104 lines, weighted move selection) have zero test coverage.

---

## 5. Untested Boundary Conditions

### Damage Pipeline
- Attack/Defense = 0 after stat stage reduction → division by zero
- Damage overflow at 997 cap → multi-level 16-bit comparison
- STAB + double super-effective (6x multiplier) → 16-bit multiplication overflow
- RandomizeDamage rejection loop with pathological RNG

### Stat Boundaries
- Speed = 0 in escape formula → division by zero
- Badge stat boost: 999 + 124 = 1123 → 999 cap comparison
- Paralysis quartering: speed 1 → 0 → clamp to 1

### Level/Experience
- Level 1 Medium Slow exp underflow → negative value
- Level 101+ with Rare Candy → `jr nc` guard (ROM-byte only)
- Exp All with single participant → divide-by-1 remainder

### Party/Inventory
- Full box capture (20 Pokémon) → shift-and-insert logic
- Empty party (1 Pokémon) faint → blackout path
- Multi-hit move KO on hit 2 of 5 → remaining hits behavior
- Disable with only 1 PP-bearing move → softlock potential

---

## 6. Test Infrastructure Analysis

### Available but unused capabilities

| Infrastructure | Status | Recommendation |
|---|---|---|
| **InputScript DSL** (`tests/src/input.rs`) | Fully built, **0 usages** | Adopt for all E2E tests — more readable than raw `press()`/`run_frames()` |
| **LinkEndpoint** (`tests/src/link.rs`) | Fully built with unit tests, **0 integration uses** | Wire up for link battle/trade protocol tests |
| **Debug utilities** (`tests/src/debug.rs`) | Built, **0 test uses** | Leverage for better test diagnostics |
| **Cycle benchmarking** | Built, only 5 usages | Expand for performance regression tracking |

### Missing infrastructure

| Gap | Impact | Effort |
|---|---|---|
| **BattleFixture** (shared battle setup) | ~50 test files independently set up battle state | Medium — extract common setup into `tests/src/battle.rs` |
| **Navigation helpers** (shared boot/menu) | E2E tests copy-paste `boot_to_main_menu()`, `navigate_oak_speech()` | Low — extract into `tests/src/navigation.rs` |
| **Tile-level visual assertions** | Golden images are brittle full-screenshot comparisons | Medium — assert specific tile IDs at tilemap coordinates |
| **SRAM fuzz testing** | No test loads corrupted save data | Medium — random SRAM → boot → verify no crash |
| **Memory watchpoints** | No way to detect unintended WRAM writes during tests | High — would require boytacean changes |

---

## 7. Virtual Time Optimization

### Current bottleneck

E2E tests are the slowest, advancing thousands of frames:

| Test File | Total Frames | Estimated Wall Time |
|---|---|---|
| `e2e_rival_battle.rs` | 7,216 | ~25 seconds |
| `e2e_gameplay.rs` | 4,644 | ~16 seconds |
| `e2e_smoke.rs` | 2,610 | ~9 seconds |
| `audio.rs` | 2,261 | ~8 seconds |

### Optimization strategies

1. **Turbo harness mode** — Disable timer, serial, DMA on top of headless PPU+APU:
   ```rust
   pub fn new_turbo() -> Self {
       let mut h = Self::new_headless();
       h.gb.set_timer_enabled(false);
       h.gb.set_serial_enabled(false);
       h.gb.set_dma_enabled(false);
       h
   }
   ```

2. **Delay routine patching** — Write `ret` (0xC9) at delay routine entry points:
   ```rust
   // Patch DelayFrame to return immediately
   h.write_mem(sym_addr("DelayFrame"), 0xC9);
   // Patch PrintLetterDelay to skip per-character wait
   h.write_mem(sym_addr("PrintLetterDelay"), 0xC9);
   ```

3. **Breakpoint stepping** — Replace `run_frames(1200)` with `step_to(next_meaningful_label)` where the next checkpoint label is known.

4. **CPU overclocking** — `gb.set_clock_freq(freq)` in boytacean allows changing virtual CPU frequency, though impact on frame-based logic needs evaluation.

### Estimated impact

| Strategy | Affected Tests | Estimated Speedup |
|---|---|---|
| Turbo mode | All headless tests | ~10-20% faster |
| Delay patching | E2E tests | **3-5x faster** (skip VBlank waits) |
| Breakpoint stepping | Tests using `run_frames(N>100)` | **5-10x faster** |
| Combined | E2E tests | **10-20x faster** (25s → 1-2s) |

---

## 8. Property-Based & Exhaustive Testing Opportunities

The existing exhaustive tests (`accuracy.rs` with 256-value sweep, `crit.rs`) are excellent models. Best candidates for extension:

### P0 — Highest value
1. **`CalculateDamage` pipeline sweep** — Inject (level, attack, defense, basePower), sweep ~500 combinations, verify against known formula. Catches any overflow/underflow in every battle's damage.
2. **`AdjustDamageForMoveType` type chart exhaustive** — Sweep all 225 type pairs (15×15), verify effectiveness multipliers. One wrong entry = broken type matchup for the entire game.
3. **`RandomizeDamage` distribution test** — 10,000 iterations, verify uniform distribution in [85%, 100%] of input damage.

### P1 — High value
4. **`ApplyBadgeStatBoosts` sweep** — 256 badge combos × ~20 stat values. Pure arithmetic, easy to make exhaustive.
5. **`TryRunningFromBattle` formula** — Sweep (playerSpeed, enemySpeed, runAttempts) triples, verify escape probability.
6. **`CheckForDisobedience` probability distribution** — 1000 iterations per scenario, verify outcome distribution.
7. **`ItemUseBall` capture rate** — Sweep (catch_rate, ball_type, status, HP_fraction), verify against Gen I capture algorithm.

---

## 9. Priority Recommendations

### Phase 1: Critical behavioral gaps (highest risk × lowest effort)

| # | What | Why | Effort |
|---|---|---|---|
| 1 | `CalculateDamage` exhaustive pipeline test | Every battle uses this; zero behavioral coverage | Medium |
| 2 | `AdjustDamageForMoveType` type chart sweep (225 combos) | Multiplicative accumulation fix changed core logic | Low |
| 3 | `RandomizeDamage` distribution test | Damage roll affects every attack | Low |
| 4 | `CalcHitChance` behavioral test | Foundation of all accuracy calculations | Medium |
| 5 | `transform.asm` behavioral test | Has documented unfixed bugs | Medium |
| 6 | Division-by-zero behavioral upgrade | Upgrade ROM-byte test to inject defense=0 | Low |
| 7 | Zero damage 0.25x clamp behavioral test | Three-way branch needs all paths executed | Low |

### Phase 2: Coverage depth improvements

| # | What | Why | Effort |
|---|---|---|---|
| 8 | Overworld behavioral tests (top 5 fixes) | 33 files, 241 tests, zero behavioral | High |
| 9 | Glitch Safety behavioral tests | MissingNo., Super Glitch, ZZAZZ at runtime | Medium |
| 10 | `TryRunningFromBattle` formula test | Complex arithmetic, overflow-prone | Medium |
| 11 | `CheckForDisobedience` behavioral test | 36 branches, all untested | Medium |
| 12 | Move effects: drain, recoil, OHKO, haze | 12/14 files untested | High |
| 13 | Golden image expansion (battle, overworld) | Only 2 golden tests exist | Medium |
| 14 | E2E expansion (gym battle, item use, save round-trip) | Current E2E covers only first 5 min | High |

### Phase 3: Infrastructure & optimization

| # | What | Why | Effort |
|---|---|---|---|
| 15 | BattleFixture shared setup module | ~50 files duplicate battle state setup | Medium |
| 16 | Navigation helpers module | E2E tests copy-paste boot sequences | Low |
| 17 | Virtual time: delay patching | 3-5x E2E speedup | Low |
| 18 | Virtual time: turbo harness mode | 10-20% faster for all headless tests | Low |
| 19 | Adopt InputScript DSL in E2E tests | Built but unused; improves readability | Medium |
| 20 | Wire up LinkEndpoint for serial tests | Built but unused; enables link testing | High |
| 21 | Trainer AI behavioral tests (Elite Four) | 24/30 AI functions untested | High |
| 22 | SRAM fuzz testing | No corrupted-save resilience tests | Medium |

---

## 10. Implementation Roadmap

### Sprint 1: Damage Pipeline (1-2 days)
- Tasks 1-3, 6-7: Exhaustive damage pipeline tests
- Reuse `accuracy.rs` exhaustive pattern (register injection + sweep)
- Expected: 5 new test files, ~50 test functions

### Sprint 2: Battle Behavioral Depth (2-3 days)
- Tasks 4-5, 10-12: Hit chance, disobedience, move effects
- Task 15: BattleFixture extraction
- Expected: 12 new test files, ~100 test functions

### Sprint 3: Overworld & Glitch Safety (2-3 days)
- Tasks 8-9: Top overworld fixes + glitch safety behavioral tests
- Task 16: Navigation helpers extraction
- Expected: 10 new test files, ~60 test functions

### Sprint 4: Infrastructure & Performance (1-2 days)
- Tasks 17-19: Virtual time optimization, InputScript adoption
- Tasks 13, 22: Golden image expansion, SRAM fuzz
- Expected: 3-5x E2E speedup, 5 new golden images

### Sprint 5: E2E & Integration (2-3 days)
- Tasks 14, 20-21: E2E expansion, link tests, AI tests
- Expected: 8 new test files, ~40 test functions

### Total estimated new tests: ~250 test functions across ~40 files
### Projected test count: ~1,200 tests (from current 958)

---

*Analysis produced by 4-agent parallel investigation: fix-auditor, harness-analyzer, coverage-mapper, asm-analyzer.*
