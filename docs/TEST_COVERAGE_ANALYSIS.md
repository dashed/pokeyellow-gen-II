# Test Coverage Analysis — Pokemon Yellow ROM Hack

Deep investigation into test coverage gaps, infrastructure capabilities, and recommendations for the pokeyellow-gen-II project.

**Date**: 2026-05-24
**Scope**: All 15 branches (14 feature + dashed/docs), 145 test files, 1,135 test functions

---

## Executive Summary

The test suite achieves **100% fix-level coverage** — every one of the 117 bug fixes has a dedicated test file. However, **73% of tests are ROM byte validation** (confirming patch bytes exist) rather than behavioral tests that execute game code. The result is a coverage paradox: broad but shallow.

**Key findings:**

| Metric | Value | Assessment |
|--------|-------|------------|
| Bug fixes with tests | 117/117 (100%) | Excellent breadth |
| Tests that execute game code | 405/1,135 (36%) | Improved after Sprint 2 |
| ROM-byte-only tests | 696/1,135 (61%) | Structural, not behavioral |
| Golden image tests | 2 (0.2%) | Massively underutilized |
| E2E gameplay tests | 10 (0.9%) | Minimal scenario coverage |
| Overworld behavioral tests | 0/241 | Largest remaining gap |
| Move effect files tested | 7/14 (50%) | +drain, ohko, recoil, haze |
| Damage pipeline tested | **Covered** (Tier 1 complete) | CalculateDamage, RandomizeDamage, CalcHitChance, type chart |
| Battle functions tested | **Covered** (Sprint 2 complete) | TryRunningFromBattle, CheckForDisobedience |
| Built but unused infrastructure | 2 modules | InputScript DSL, LinkEndpoint |

**Bottom line**: The Tier 1 and Sprint 2 gaps have been closed — the damage pipeline (CalculateDamage, CalcHitChance, RandomizeDamage, type chart, TransformEffect_), escape formula (TryRunningFromBattle), obedience system (CheckForDisobedience), and 4 move effects (drain, OHKO, recoil, haze) now have exhaustive behavioral tests. A shared BattleFixture module reduces future test setup boilerplate. The remaining gaps are overworld behavioral coverage (33 files, all ROM-byte only), move effects (7/14 untested), and glitch safety runtime verification.

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
ROM Byte Validation:  ████████████████████████████████████████  98 files (67%)  696 tests (61%)
Emulator State Test:  ██████████████████         37 files (26%)  405 tests (36%)
E2E Gameplay Test:    ██                          3 files  (2%)   10 tests  (0.9%)
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
| **Battle Engine** | 159 | 270 | 0 | 0 | **429** |
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

### Tier 1: CRITICAL — Complex functions (**RESOLVED**)

The following Tier 1 gaps have been closed with 71 new behavioral tests:

| Function | File | Status | Tests |
|---|---|---|---|
| **`CalculateDamage`** | `core.asm:4655` | **COVERED** | `calculate_damage.rs` — 9 tests: 3,920-combo formula sweep, min/max caps, Explosion defense halving |
| **`RandomizeDamage`** | `core.asm:5893` | **COVERED** | `randomize_damage.rs` — 7 tests: damage ≤1 unchanged, range [85%,100%], 2000-trial uniformity |
| **`CalcHitChance`** | `core.asm:5821` | **COVERED** | `calc_hit_chance.rs` — 9 tests: all 169 stage combos swept, boundary clamping [1,255] |
| **`AdjustDamageForMoveType`** | `core.asm:5471` | **COVERED** | `type_chart.rs` — 13 tests: all 225 single-type pairs + 12 dual-type interactions |
| **`TransformEffect_`** | `transform.asm` | **COVERED** | `transform_effect.rs` — 16 tests: species/types/stats/moves/DVs copy, PP=5, 2 INVULNERABLE bugs documented |
| **Division-by-zero clamp** | `core.asm:4376` | **COVERED** | `division_zero.rs` — +3 behavioral tests: defense=0 clamp verified at runtime |
| **Zero damage 0.25x clamp** | `core.asm:5471` | **COVERED** | `zero_damage.rs` — +6 behavioral tests: all 3 paths (normal, immune, 0.25x→1) |

### Remaining CRITICAL — Sprint 2 resolved TryRunningFromBattle and CheckForDisobedience

| Function | File | Status | Tests |
|---|---|---|---|
| **`TryRunningFromBattle`** | `core.asm:1635` | **COVERED** | `try_running.rs` — 14 tests: deterministic paths, formula sweep, threshold overflow |
| **`CheckForDisobedience`** | `core.asm:4176` | **COVERED** | `disobedience.rs` — 21 tests: badge thresholds, OT ID, all outcome paths |
| **`SelectEnemyMove`** | `core.asm:3231` | **HIGH** | Untested. 93 lines, 18 branches. Move selection with Metronome/Mirror Move. |
| **`GainExperience`** | `experience.asm` | **HIGH** | Untested. ~200 lines. Full exp distribution, level-ups, move learning. |

### Move Effects: 7 of 14 implementations still untested

`substitute.asm`, `heal.asm`, `transform.asm`, `drain_hp.asm`, `one_hit_ko.asm`, `recoil.asm`, and `haze.asm` have test coverage:

| Untested File | Risk | Key Concern |
|---|---|---|
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

### Damage Pipeline (partially resolved)
- ~~Attack/Defense = 0 after stat stage reduction → division by zero~~ **TESTED** (division_zero.rs behavioral)
- ~~Damage overflow at 997 cap → multi-level 16-bit comparison~~ **TESTED** (calculate_damage.rs)
- STAB + double super-effective (6x multiplier) → 16-bit multiplication overflow
- ~~RandomizeDamage rejection loop with pathological RNG~~ **TESTED** (randomize_damage.rs distribution)

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
| ~~**BattleFixture**~~ | ~~shared battle setup~~ | **DONE** — `tests/src/battle.rs` with `BattleFixture::new("Label")`, Deref to TestHarness, WRAM helpers |
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

### Phase 1: Critical behavioral gaps — **COMPLETE**

All 7 items delivered (71 new tests, 5 new files + 2 upgraded):

| # | What | Status | Tests |
|---|---|---|---|
| 1 | `CalculateDamage` exhaustive pipeline test | **DONE** — `calculate_damage.rs` | 9 |
| 2 | `AdjustDamageForMoveType` type chart sweep (225 combos) | **DONE** — `type_chart.rs` | 13 |
| 3 | `RandomizeDamage` distribution test | **DONE** — `randomize_damage.rs` | 7 |
| 4 | `CalcHitChance` behavioral test | **DONE** — `calc_hit_chance.rs` | 9 |
| 5 | `transform.asm` behavioral test | **DONE** — `transform_effect.rs` | 16 |
| 6 | Division-by-zero behavioral upgrade | **DONE** — `division_zero.rs` +3 | 3 |
| 7 | Zero damage 0.25x clamp behavioral test | **DONE** — `zero_damage.rs` +6 | 6 |

Branch placement: Items 1, 3, 4, 5 on `dashed/tests`; Items 2, 6, 7 on `dashed/battle-bugs`.

### Phase 2: Coverage depth improvements (next priority)

| # | What | Status | Effort |
|---|---|---|---|
| 8 | Overworld behavioral tests (top 5 fixes) | 33 files, 241 tests, zero behavioral | High |
| 9 | Glitch Safety behavioral tests | MissingNo., Super Glitch, ZZAZZ at runtime | Medium |
| 10 | ~~`TryRunningFromBattle` formula test~~ | **DONE** — `try_running.rs` (14 tests) | ~~Medium~~ |
| 11 | ~~`CheckForDisobedience` behavioral test~~ | **DONE** — `disobedience.rs` (21 tests) | ~~Medium~~ |
| 12 | ~~Move effects: drain, recoil, OHKO, haze~~ | **DONE** — 4 files (47 tests) | ~~High~~ |
| 13 | Golden image expansion (battle, overworld) | Only 2 golden tests exist | Medium |
| 14 | E2E expansion (gym battle, item use, save round-trip) | Current E2E covers only first 5 min | High |

### Phase 3: Infrastructure & optimization

| # | What | Why | Effort |
|---|---|---|---|
| 15 | ~~BattleFixture shared setup module~~ | **DONE** — `tests/src/battle.rs` (5 unit tests) | ~~Medium~~ |
| 16 | Navigation helpers module | E2E tests copy-paste boot sequences | Low |
| 17 | Virtual time: delay patching | 3-5x E2E speedup | Low |
| 18 | Virtual time: turbo harness mode | 10-20% faster for all headless tests | Low |
| 19 | Adopt InputScript DSL in E2E tests | Built but unused; improves readability | Medium |
| 20 | Wire up LinkEndpoint for serial tests | Built but unused; enables link testing | High |
| 21 | Trainer AI behavioral tests (Elite Four) | 24/30 AI functions untested | High |
| 22 | SRAM fuzz testing | No corrupted-save resilience tests | Medium |

---

## 10. Implementation Roadmap

### Sprint 1: Damage Pipeline — **COMPLETE** (2026-05-24)
- ~~Tasks 1-3, 6-7: Exhaustive damage pipeline tests~~ **DONE**
- 5 new test files + 2 upgraded, 71 new test functions
- Pattern: register injection + sweep (reused accuracy.rs pattern)
- Branch: `dashed/tests` (4 vanilla) + `dashed/battle-bugs` (3 fix-specific)

### Sprint 2: Battle Behavioral Depth — **COMPLETE** (2026-05-24)
- ~~Tasks 10-12: Escape formula, disobedience, move effects~~ **DONE**
- ~~Task 15: BattleFixture extraction~~ **DONE**
- 7 new files (6 test + 1 infrastructure), 87 new test functions
- Pattern: fixture setup + label stepping for display-calling functions
- Branch: `dashed/tests` (all vanilla function tests + infrastructure)
- Files: `try_running.rs` (14), `disobedience.rs` (21), `one_hit_ko_effect.rs` (10), `drain_hp_effect.rs` (11), `recoil_effect.rs` (12), `haze_effect.rs` (14), `battle.rs` (5)

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

### Remaining estimated new tests: ~95 test functions across ~25 files
### Current test count: 1,135 (up from 1,048 after Sprint 2)
### Projected final count: ~1,230 tests

---

*Analysis produced by 4-agent parallel investigation (fix-auditor, harness-analyzer, coverage-mapper, asm-analyzer). Sprint 1 implemented by 4-agent parallel team (damage-pipeline, type-chart, rng-hitchance, transform-test). Sprint 2 implemented by 4-agent parallel team (escape-formula, disobedience, move-effects, battle-fixture). Updated 2026-05-24.*
