# Vanilla Bugs & Glitches

Known bugs inherited from the original Pokemon Yellow (Gen 1) codebase. This document tracks which ones we've fixed, which ones we've intentionally left, and which ones could be fixed in the future.

Sourced from: [Glitch City Wiki](https://glitchcity.wiki/wiki/Category:Generation_I_glitches), [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I), and direct code audit.

## Fixed

### 1/256 miss glitch

**File**: `engine/battle/core.asm` (MoveHitTest.doAccuracyCheck)

The accuracy check used `random < accuracy` (strictly less than). If the random number is 255, it can never be less than any accuracy value (max 255), so the move always misses — even 100% accuracy moves miss 1/256 of the time (~0.4%).

**Our fix**: Optimal rounding with three-way logic:
- N=255 → always hit (bypass RNG with `inc a / ret z`)
- N≥128, random==N → hit (`≤` comparison, closer to ideal N/255)
- N<128, random==N → miss (`<` comparison, closer to ideal N/255)

Uses `inc a` instead of `cp $FF` to test for 255 — same Z flag behavior (wraps to 0), saves 1 ROM byte. Safe because `call BattleRandom` immediately overwrites A.

This is superior to the [standard community fix](https://glitchcity.wiki/wiki/1/256_miss_glitch) which uses `≤` for all values. Our maximum error from ideal is 0.20% vs 0.39%.

**Tests**: 11 tests in `tests/tests/accuracy.rs` including exhaustive verification of all 65,280 combinations, plus 3 link battle regression tests in `tests/tests/link_regression.rs`.

**References**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#1/256_miss_glitch), [Glitch City Wiki](https://glitchcity.wiki/wiki/1/256_miss_glitch)

### 1/256 critical hit miss

**File**: `engine/battle/core.asm` (CriticalHitTest.SkipHighCritical)

Same class of bug as the accuracy glitch. The critical hit check uses `random < crit_rate` (strictly less than). When the crit rate is 255 (e.g. a fast Pokemon using Slash, Razor Leaf, or Crabhammer), a random value of 255 gives `255 < 255 = false`, preventing a guaranteed critical hit 1/256 of the time.

**Our fix**: `ld a, b / inc a / jr z, .criticalHit` before the RNG call — crit rate 255 always crits without consulting BattleRandom. Same `inc a` optimization as the accuracy fix.

**Tests**: 6 tests in `tests/tests/crit.rs` verifying the fix and normal crit behavior using link battle deterministic RNG.

**Reference**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#1/256_miss_glitch) (same root cause as the 1/256 miss glitch)

### Focus Energy / Dire Hit bug

**File**: `engine/battle/core.asm` (CriticalHitTest, `.calcCriticalHitProbability`)

Focus Energy and Dire Hit were intended to quadruple the critical hit rate, but due to a branch condition bug (`jr nz` instead of `jr z`), they quartered it instead. The `srl b` (÷2) path ran when Focus Energy was active, and the `sla b` (×2) path ran without it — the exact opposite of the design intent.

**Our fix**: Single byte change — swap `jr nz` to `jr z`. The `sla b` (×2) path now runs for Focus Energy, and `srl b` (÷2) runs for normal. This gives Focus Energy a clean ×4 multiplier on the final crit rate. The fix preserves the identical ROM layout (no address shifts).

**Note**: This lowers the "normal" (non-Focus-Energy) crit rate from what Gen 1 players experienced, because the bugged code was using the intended Focus Energy rate as the normal rate.

| Case | No FE (fixed) | With FE (fixed) | Ratio |
|------|---------------|-----------------|-------|
| Normal move | speed/8 | speed/2 | ×4 |
| High-crit move | speed | speed×4 (cap 255) | ×4 |

**Tests**: 6 tests in `tests/tests/crit.rs` verifying Focus Energy quadruples crit rate for normal and high-crit moves, plus edge cases.

**Reference**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Critical_hit_ratio_error)

### Substitute 0 HP bug

**File**: `engine/battle/move_effects/substitute.asm` (SubstituteEffect_)

The original code only checked for underflow (carry flag) when subtracting quarterHP from currentHP. If `currentHP == quarterHP`, the subtraction produces 0 with no carry, so the substitute was created leaving the Pokemon alive with 0 HP.

**Our fix**: After the carry check, added `ld e,a / or d / jr z, .notEnoughHP` — saves the high byte in E (free after the HP offset calculation), ORs both result bytes to check if remaining HP is exactly 0, then rejects the substitute if so. Restores A from E before continuing. 5 extra bytes in bank $05.

**Tests**: 8 tests in `tests/tests/substitute.rs` covering exact-quarter rejection, below-quarter rejection, above-quarter acceptance, full HP, rounding, 1 HP edge case, and high HP values.


### Dual-type move effectiveness message misreported

**File**: `engine/battle/core.asm` (AdjustDamageForMoveType.matchingPairFound)

When a move hits a dual-type Pokemon, the type effectiveness loop iterates over the `TypeEffects` table and applies damage multipliers for each matching type. The damage calculation is correct (multiplicative), but `wDamageMultipliers` — which determines the "super effective" / "not very effective" message — was overwritten by each type match instead of accumulated. The second type's effectiveness replaced the first, so the displayed message only reflected the last match in table order.

Example: Grass vs Gyarados (Water/Flying) — damage is correctly neutral (SE×NVE = ×1), but the game shows "It's not very effective..." because the Grass-vs-Flying entry (NVE) appears later in the table and overwrites the Grass-vs-Water entry (SE).

**Our fix**: Replace the overwrite (`add b / ld [wDamageMultipliers], a`) with multiplicative accumulation using bit-testing: `bit 4, c` distinguishes SUPER_EFFECTIVE ($14, bit 4 set) from NOT_VERY_EFFECTIVE ($05, bit 4 clear), then `sla a` (double) or `srl a` (halve) the accumulated value. Immune ($00) zeroes the accumulator via early exit. 19 extra bytes — 6 bytes smaller than the standard community fix which uses two separate load-mask-shift blocks.

**Tests**: 12 tests in `tests/tests/effectiveness.rs` covering SE+NVE cancellation (Gyarados), 4× SE, 4× NVE, immunity override, STAB preservation, and mono-type correctness.

**Reference**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Dual-type_damage_misinformation)

### Psychic/Psywave/Night Shade animation top 3 lines not wiggling

**File**: `engine/battle/animations.asm` (`AnimationWavyScreen`)

The wavy screen effect uses an HBlank polling loop to set `rSCX` per-scanline, creating a horizontal wave distortion. After VBlank ends and scanline 0 starts rendering, the polling loop is still waiting for its first HBlank — by the time it catches mode 0, several scanlines have already been drawn without the wave offset. This leaves the top ~3 screen lines static while the rest wiggles.

**Our fix**: At the top of `.loop`, write the current wave offset to `hSCX` (the shadow register that the VBlank handler copies to `rSCX`). This ensures scanline 0 renders with the correct wave offset. After the animation loop, clear `hSCX` to 0 so subsequent VBlanks don't shift the screen. +5 bytes in bank $1E.

**Tests**: 5 tests in `tests/tests/wavy_screen.rs` verifying hSCX is set to positive, zero, and negative offsets at `.loop`, cleared after the animation, and ROM bytes confirming the fix instructions.

### Slide animation tearing (fainted mon / trainer pic)

**File**: `engine/battle/core.asm` (`SlideDownFaintedMonPic`, `SlideTrainerPicOffScreen`)

When a Pokémon faints or a trainer pic slides on/off screen, the tilemap is modified row-by-row in RAM, then `DelayFrames` is called to pace the animation. But `hAutoBGTransferEnabled` remains non-zero throughout, so the VBlank handler's `AutoBgMapTransfer` routine transfers the partially-modified tilemap to VRAM mid-copy. This causes visible screen tearing during battle transitions — switching out, fainting, trainer dialogue, etc.

**Our fix**: Disable `hAutoBGTransferEnabled` (`xor a / ldh [hAutoBGTransferEnabled], a`) before each slide step's tilemap modification, then re-enable it (`ld a, 1 / ldh [hAutoBGTransferEnabled], a`) after the step is complete but before `DelayFrames`. This ensures VBlank only transfers fully-assembled frames. +7 bytes per function, +14 bytes total in bank $0F.

**Tests**: 8 tests in `tests/tests/slide_tearing.rs` — bank $0F check (both functions), disable before `.rowLoop` (both), re-enable after tilemap modification (both), re-enable before `DelayFrames` (both), negative test (no `DelayFrames` call while BG transfer is disabled in either function).

### Trainer Card transition garbage on DMG (IPS LCD)

**File**: `engine/menus/start_sub_menus.asm` (`StartMenu_TrainerInfo`)

On a DMG with a modern IPS LCD screen mod, the Trainer Card screen can show brief garbage when transitioning in and out. The faster IPS LCD response time (~1 ms vs ~40 ms for the original LCD) makes partially-loaded tiles visible during palette transitions that were hidden on the original slow-fade screen.

**Our fix**: Add `Delay3` (3-frame delay) at two points, gated on `wOnSGB == 0` (skipped on SGB which has its own timing): after `call RunPaletteCommand` before the card becomes visible, and after `farcall DrawStartMenu` before restoring the map palette. The `ld a, [wOnSGB] / and a / call z, Delay3` pattern uses a conditional call to avoid unnecessary delays on SGB. +14 bytes in bank $04.

**Tests**: 8 tests in `tests/tests/trainer_card_dmg.rs` — bank $04 check, entry delay after RunPaletteCommand, entry delay before GBPalNormal, entry delay uses `call z` ($CC), exit delay before LoadGBPal, exit delay after ReloadMapData, exit delay uses `call z`, exactly 2 delay patterns in function.

### Double Edge opponent animation uses wrong mirror type

**File**: `data/battle_anims/subanimations.asm` (`Subanim_0CirclesCentering`)

When the player uses Double Edge, circular orbs animate inward from the four corners of the player's sprite. However, when the opponent uses it, the orbs appear at incorrect positions instead of mirroring properly. The subanimation `Subanim_0CirclesCentering` specifies `SUBANIMTYPE_COORDFLIP` (which swaps X/Y coordinates) instead of `SUBANIMTYPE_HVFLIP` (which mirrors both horizontally and vertically). The coordinate flip produces nonsensical positions when the animation is reflected for the opponent's sprite.

**Our fix**: Change the subanimation type from `SUBANIMTYPE_COORDFLIP` to `SUBANIMTYPE_HVFLIP`. One-byte change in bank $1E: the header byte changes from `$66` ((3<<5)|6) to `$26` ((1<<5)|6).

**Tests**: 8 tests in `tests/tests/double_edge_anim.rs` — bank $1E check, address in banked range, header uses HVFLIP not COORDFLIP, type bits are HVFLIP, frame count is 6, first/second frame entry data integrity, total data length is 19 bytes (verified by checking the next subanimation header).

### Pitch slide borrows from wrong frequency high byte

**File**: `audio/engine_1.asm` (`Audio1_InitPitchSlideVars.targetFrequencyGreater`)

When the audio engine computes the frequency difference for pitch slides, the code at `.targetFrequencyGreater` borrows from the high byte of the **current** frequency instead of the **target** frequency. When the low byte subtraction produces a carry (current_lo > target_lo), the borrow is applied to the wrong source, making the result $200 (512) greater than intended in the 11-bit frequency value.

**Our fix**: Load `wChannelPitchSlideTargetFrequencyHighBytes` before `sbc b` so the borrow applies to the target high byte. Replaces the buggy 10-byte sequence (`ld a, d / sbc b / ld d, a` + separate target_hi load/sub) with a correct 8-byte sequence (`ld hl, target_hi / add hl, bc / ld a, [hl] / sbc b / sub d / ld d, a`). −2 bytes in bank $02.

**Tests**: 8 tests in `tests/tests/pitch_slide.rs` — bank $02 check, address in banked range, target high byte load found in `.targetFrequencyGreater`, `sbc b` follows target load, `sub d` follows `sbc b`, `ld d, a` saves result, no buggy `ld a, d` + `sbc b` pattern, fix sequence is 8 bytes followed by `ld b, 0`.

### Route 8 Super Nerd truncated "chem" text

**File**: `text/Route8.asm` (`_Route8SuperNerd1BattleText`)

The Super Nerd on Route 8 says "how's your chem?" — the word "chemistry" was truncated to "chem" to fit on one line, producing awkward phrasing. The text box has room for the full word if the sentence is reflowed across a paragraph break.

**Our fix**: Change `cont "how's your chem?"` to `cont "how's your"` / `para "chemistry grade?"`. The text now reads naturally across two text box screens. +3 bytes in text bank $28.

**Tests**: 8 tests in `tests/tests/route8_text.rs` — bank $28 check, banked range, TX_START at label, CONT byte present, PARA byte follows "how's your", "chemistry" encoded bytes follow PARA, DONE after question mark, no old truncated "chem?" + DONE pattern.

### Psywave link battle desync

**File**: `engine/battle/core.asm` (`ApplyAttackToPlayerPokemon`)

Psywave's damage is a random value in [1, level×1.5). The player's code (`ApplyAttackToEnemyPokemon`) correctly rejects 0 from the RNG with `and a / jr z, .loop`, but the enemy's code (`ApplyAttackToPlayerPokemon`) accepted 0 as valid damage. In link battles, both Game Boys run the calculation using a shared RNG list — if the RNG produces 0, one side retries (consuming an extra value) while the other doesn't, desyncing all subsequent RNG operations.

**Our fix**: Added `and a / jr z, .loop` after `call BattleRandom` in the enemy path to match the player path. +3 bytes in bank $0F.

**Tests**: 8 tests in `tests/tests/psywave.rs` verifying both sides reject 0, accept valid values, reject values ≥ max, and consume the same number of RNG values for identical inputs.

**Reference**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Psywave_desynchronization)

### Psywave infinite loop (levels 0, 1, and 171)

**File**: `engine/battle/core.asm` (`ApplyAttackToEnemyPokemon`, `ApplyAttackToPlayerPokemon`)

Psywave generates random damage in [1, level×1.5). For levels 0, 1, or 171 (byte overflow: 171×1.5 = 256 → 0), the upper bound is ≤1, making the valid range empty. The RNG rejection loop never terminates, softlocking the game. While only obtainable via glitches in normal gameplay, this is a safety concern for any glitch-accessible Pokémon.

**Our fix**: After computing `b = level * 1.5`, clamp B to minimum 2 with `cp 2 / jr nc, .loop / ld b, 2`. This ensures the range [1, 2) = {1} always contains a valid value. Applied to both player-side and enemy-side Psywave paths. +6 bytes per side, +12 bytes total in bank $0F.

**Tests**: 5 tests in `tests/tests/psywave_loop.rs` — bank $0F check, player and enemy clamp sequences (`cp 2 / jr nc, .loop / ld b, 2`), `ld b, 2` immediately precedes `.loop` on both sides.

**Reference**: [Bulbapedia — Psywave infinite loop](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Psywave_infinite_loop)

### Red bar glitch (low HP alarm overrides battle sounds)

**Files**: `engine/battle/core.asm` (`DrawPlayerHUDAndHPBar`), `audio/low_health_alarm.asm` (`Music_DoLowHealthAlarm`), `ram/wram.asm`

The low HP alarm writes directly to sound channel 1 hardware registers (rAUD1SWEEP) every frame, overriding all battle move sound effects and suppressing animations. This is because the Game Boy has limited audio channels and the alarm bypasses the normal audio engine.

**Our fix**: Add a beep counter (`wLowHealthAlarmCounter`) initialized to 4 when the alarm activates. `Music_DoLowHealthAlarm` decrements the counter each beep cycle; at 0, the alarm auto-disables. The alarm re-enables on the next HUD redraw (each turn or HP change), creating Gen II-like behavior: brief beeping then silence during move execution. The check `bit BIT_LOW_HEALTH_ALARM, [hl] / jr nz` prevents resetting the counter while the alarm is still active. +8 bytes in bank $0F, +9 bytes in audio bank, +1 byte WRAM.

**Tests**: 6 tests in `tests/tests/red_bar.rs` — bank $0F check, `bit 7, [hl]` checks alarm-already-on before counter reset, counter initialized to 4 with `ld a, 4 / ld [wLowHealthAlarmCounter], a`, `.alarmAlreadyOn` is `ret`, alarm handler decrements counter pattern found, `jr z` targets `.disableAlarm`.

**Reference**: [Bulbapedia — Red bar glitch](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Red_bar_glitch) | [Glitch City Wiki — Red-bar sound effect manipulation](https://glitchcity.wiki/wiki/Red-bar_sound_effect_manipulation)

### Stat modification errors (badge boost stacking, wrong-target status penalties)

**File**: `engine/battle/effects.asm` (`StatModifierUpEffect`, `StatModifierDownEffect`)

Three bugs triggered whenever a stat stage is modified:

1. **Badge boost stacking**: `ApplyBadgeStatBoosts` is called after every stat modification, re-applying the 1/8 badge boost to ALL stats — including those already boosted. This stacks multiplicatively up to the 999 cap.
2. **Wrong-target paralysis penalty**: `QuarterSpeedDueToParalysis` applies the Speed quarter to the Pokémon whose turn it is NOT (the opponent of the move user), instead of the paralyzed Pokémon.
3. **Wrong-target burn penalty**: `HalveAttackDueToBurn` similarly halves the wrong Pokémon's Attack.

**Our fix**: Remove all three erroneous calls from both `StatModifierUpEffect` and `StatModifierDownEffect`. The individual stat is already correctly recalculated from `unmodified_stat × stage_ratio` at `.recalculateStat`. Badge boosts and status penalties are already baked into stats from battle initialization and status infliction respectively. -22 bytes (saves space).

**Tests**: 5 tests in `tests/tests/stat_mod_errors.rs` — bank $0F check, stat-up block is `ld hl / call PrintText / ret` (no badge/status calls), stat-down block is same, `ApplyBadgeStatBoosts` address absent from both blocks.

**Reference**: [Bulbapedia — Stat modification errors](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Stat_modification_errors) | [Glitch City Wiki — Stat modification glitches](https://glitchcity.wiki/wiki/Stat_modification_glitches)

### Fly/Dig invulnerability persists through full paralysis

**File**: `engine/battle/core.asm` (`CheckPlayerStatusConditions`, `CheckEnemyStatusConditions`)

When full paralysis or confusion self-hit prevents a Pokemon from completing the second turn of Fly or Dig, the INVULNERABLE bit (bit 6 of wBattleStatus1) stays set permanently. All opponent moves then miss until the status is cleared by using Fly/Dig again, making the Pokemon effectively invincible.

**Our fix**: Added `(1 << INVULNERABLE)` to the AND bitmask in `.MonHurtItselfOrFullyParalysed` for both player and enemy sides. This changes the immediate operand from $CC to $8C — a zero-byte fix (no ROM growth). The confusion path already clears INVULNERABLE via `and 1 << CONFUSED` before reaching this code, so the fix only changes behavior for the paralysis case.

**Tests**: 8 tests in `tests/tests/invulnerable.rs` verifying ROM bytes ($E6 $8C), behavioral clearing of INVULNERABLE for both player and enemy, mask correctness for all status bits, and compatibility with non-Fly charging moves (Solar Beam).

**Reference**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Invulnerability_glitch)
### Healing moves fail when HP is 255 or 511 below max

**File**: `engine/battle/move_effects/heal.asm` (`HealEffect_`)

Recover, Softboiled, and Rest check if the user's HP is already full before healing. The 16-bit HP comparison used `cp [hl]` for the high byte then `sbc [hl]` for the low byte, but only checked the Z flag after `sbc`. When `maxHP - currentHP` is exactly 255 or 511, the `sbc` result happens to be 0 despite the high bytes differing, causing the move to incorrectly fail with "But it failed!".

**Our fix**: Replaced `cp [hl]` with `sub [hl]` and saved the high byte difference in C with `ld c, a`. After `sbc [hl]`, added `or c` so Z is only set when both byte differences are 0 (true equality). +2 bytes in bank $3D.

**Tests**: 10 tests in `tests/tests/heal.rs` verifying ROM bytes (`sub` opcode $96 replacing `cp` $BE, `ld c,a` and `or c` present), behavioral tests for the 255-gap and 511-gap bug scenarios, full-HP failure, and various partial-HP success cases.

**Reference**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#HP_recovery_move_failure)
### Switch-out messages do not account for HP underflow

**File**: `engine/battle/common_text.asm` (`PlayerMon2Text` text_asm callback)

When switching Pokemon, the game computes a damage percentage based on `lastSwitchInHP - currentHP` to select an appropriate message ("Enough!", "Come back!", "OK!", or "Good!"). This 16-bit unsigned subtraction underflows when the enemy gained HP since switch-in (e.g. via healing moves), producing a garbage percentage that can display an incorrect message like "Good!" when no damage was dealt.

**Our fix**: After the `sbc b` high-byte subtraction, check the carry flag with `jr c, .gainedHP`. If set (underflow), skip the Multiply/Divide computation entirely and return `EnoughText` directly (pop saved registers + ret). +8 bytes in bank $3D (2 inline + 6 at end).

**Tests**: 9 tests in `tests/tests/switch_message.rs` verifying ROM bytes (`push de` entry, `sbc b` + `jr c` fix pattern), underflow scenarios (small/large HP gain, byte boundary crossing), no-damage case, and all four normal message thresholds (1-29%, 30-69%, 70%+).


### Haze freeze / Hyper Beam recharge softlock

**File**: `engine/battle/effects.asm` (`FreezeBurnParalyzeEffect.freeze2`)

When an enemy freezes the player during Hyper Beam recharge, the NEEDS_TO_RECHARGE bit (bit 5 of `wPlayerBattleStatus2`) is not cleared — unlike when the player freezes the enemy (`.freeze1` already calls `ClearHyperBeam`). If the enemy then uses Haze to cure the freeze, `wPlayerSelectedMove` is set to $FF (CANNOT_MOVE). On the next turn, `ExecutePlayerMove` bails out at the CANNOT_MOVE check before reaching `CheckPlayerStatusConditions`, so the `.HyperBeamCheck` that clears NEEDS_TO_RECHARGE is never reached. The player is permanently locked out of selecting moves.

**Our fix**: Added `call ClearHyperBeam` at the start of `.freeze2`, matching the existing call in `.freeze1`. This clears NEEDS_TO_RECHARGE when freeze is applied, preventing the softlock even if Haze later sets the selected move to $FF. +3 bytes in bank $0F.

**Tests**: 6 tests in `tests/tests/haze_freeze.rs` verifying ROM bytes (`call ClearHyperBeam` at both `.freeze2` and `.freeze1`), behavioral clearing of NEEDS_TO_RECHARGE when enemy freezes player, preservation of other status bits, harmless no-op when no recharge is set, and symmetry with player-freezes-enemy path.

**Reference**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Hyper_Beam_+_Freeze_permanent_helplessness)

### CooltrainerF AI always switches instead of 25% chance

**File**: `engine/battle/trainer_ai.asm` (`CooltrainerFAI`)

The CooltrainerF AI routine is missing a `ret nc` after `cp 25 percent + 1`. Every other trainer AI using this pattern (JugglerAI, GiovanniAI, CooltrainerMAI, MistyAI, etc.) correctly returns early 75% of the time, but CooltrainerFAI always falls through to the HP check and switching logic. This makes CooltrainerF deterministically switch at 10-20% HP instead of only 25% of the time.

**Our fix**: Uncommented the `ret nc` instruction after the `cp 25 percent + 1`, restoring the intended 25% probability gate. +1 byte in bank $0E.

**Tests**: 7 tests in `tests/tests/cooltrainer_ai.rs` verifying ROM bytes (`cp $40` + `ret nc` at CooltrainerFAI, CooltrainerMAI, and JugglerAI for symmetry), behavioral tests confirming `ret nc` is taken when random >= threshold and not taken when random < threshold (with boundary cases).

### Transformed Pokémon assumed to be Ditto when catching

**File**: `engine/items/item_effects.asm` (`ItemUseBall`)

When catching a transformed wild Pokémon, the code checked the TRANSFORMED bit in `wEnemyBattleStatus3` and, if set, overwrote `wEnemyMonSpecies2` with DITTO ($4C). This assumed only Ditto could be transformed, but a non-Ditto wild Pokémon could use Transform via Mirror Move. Catching it would yield a Ditto instead of the actual species.

**Our fix**: Removed the `ld a, DITTO / ld [wEnemyMonSpecies2], a` and changed `jr z, .notTransformed` to `jr nz, .skip6`. When TRANSFORMED is set, `wEnemyMonSpecies2` already holds the correct original species — Transform only overwrites `wEnemyMonSpecies`, not `wEnemyMonSpecies2`. -7 bytes in bank $03.

**Tests**: 5 tests in `tests/tests/transform_catch.rs` verifying ROM bytes (no Ditto assignment, `jr nz` targets `.skip6`), behavioral preservation of original species through the transform check, and correct `.notTransformed` path (DVs saved, TRANSFORMED bit set).

**Reference**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_Transform_glitches#Transform_assumption_oversight)

### Ghost battle marks real species as seen in Pokédex

**File**: `engine/battle/core.asm` (`LoadEnemyMonData`)

In Pokémon Tower without the Silph Scope, wild Pokémon appear as "Ghost". However, `LoadEnemyMonData` unconditionally marks the real species as "seen" in the Pokédex via `FlagActionPredef`, revealing what Pokémon is behind the Ghost before the player identifies it.

**Our fix**: Added `call IsGhostBattle / jr z, .skipPokedexSeen` before the Pokédex seen flag block. When `IsGhostBattle` returns Z (wild battle in Pokémon Tower without Silph Scope), the `FlagActionPredef` call is skipped. +5 bytes in bank $0F.

**Tests**: 14 tests in `tests/tests/ghost_pokedex.rs` verifying ROM bytes and behavioral tests (see below).

**Reference**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Ghost_identity_unveiling)

### Ghost sprite revealed on party menu return

**File**: `engine/battle/core.asm` (sprite reload after party menu/bag)

In a ghost battle in Pokémon Tower, returning from the party menu or bag reloads the enemy sprite from `wEnemyMonSpecies` — the real species, not the Ghost. This visually reveals the Pokémon behind the Ghost without the Silph Scope.

**Our fix**: Added `call IsGhostBattle / jr nz, .notGhostReload / ld a, MON_GHOST` before the existing `GetMonHeader` call. In ghost battles, `MON_GHOST` ($B8) is substituted so `GetMonHeader` loads the ghost sprite and dimensions. +9 bytes in bank $0F.

**Tests**: Shares the 14 tests in `tests/tests/ghost_pokedex.rs` — 4 ROM byte tests for the sprite reload fix site (`call IsGhostBattle`, `jr nz`, `ld a, MON_GHOST`, `ld a, [wEnemyMonSpecies]`), 2 ROM byte tests for the Pokédex fix, and 8 behavioral tests for `IsGhostBattle`.

**Reference**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Ghost_identity_unveiling)
### Exp. All experience distribution

**File**: `engine/battle/core.asm` (FaintEnemyPokemon, around line 822)

When the player has Exp. All, `wEnemyMonBaseStats` is halved and `GainExperience` is called twice. The first call (for battle participants) triggers `DivideExpDataByNumMonsGainingExp`, which divides `wEnemyMonBaseStats` **in place** by the participant count. The second call (Exp. All distribution to all party members) then receives `(base/2/numFighters)` instead of the correct `(base/2)`, reducing overall experience gained.

**Fix**: Before the first `GainExperience` call, count participants via popcount on `wPartyGainExpFlags`. After the call, multiply each of the 7 `wEnemyMonBaseStats` bytes back by the participant count. +53 bytes in bank $0F.

**Tests**: 12 tests in `tests/tests/exp_all.rs` — ROM byte verification (halve loop count, srl [hl], multiply inner loop, cp 2 skip branch, particle count read) and behavioral tests (multiply restores values for 2/3 participants, popcount, skip with 1 participant, gain exp flags for 4/6 members).

**Reference**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Exp._All_oversight)
### Status-curing items remove stat modifiers

**File**: `engine/items/item_effects.asm` (ItemUseMedicine, `.cureStatusAilment` and Full Restore HP path)

Using a status-curing item (Burn Heal, Parlyz Heal, Full Heal, Full Restore, etc.) on the active battle Pokémon copies raw party stats into `wBattleMonStats`, wiping out all stat stage modifiers (+1 through +6 / −1 through −6) and badge boosts. The useless `predef DoubleOrHalveSelectedStats` call that followed did nothing (the stat selection variables are always 0). The Full Restore HP path also failed to clear the BADLY_POISONED flag in `wPlayerBattleStatus3`.

**Fix**: Replace the CopyData + predef with a shared subroutine `.reapplyStatModsAfterCure` (26 bytes) that: (1) clears BADLY_POISONED, (2) calls `CalculateModifiedStats` to reapply stat stage ratios from `wPlayerMonStatMods`, and (3) calls `ApplyBadgeStatBoosts`. Both code paths (individual cures + Full Restore HP) now call this subroutine. +9 bytes in bank $03.

**Tests**: 14 tests in `tests/tests/status_cure.rs` — ROM byte verification (callfar targets, res instruction, call/ret, push/pop de in Path 2, no CopyData/predef) and behavioral tests (BADLY_POISONED cleared, neutral stats restored, +2 stage applied, −1 stage applied, badge boost applied, combined stages + badges, other status3 bits preserved).


## Not fixed (intentional game design)

### X Accuracy bypasses all accuracy checks

**File**: `engine/battle/core.asm` (MoveHitTest, lines with `USING_X_ACCURACY`)

X Accuracy sets a status bit that causes MoveHitTest to `ret nz` immediately — bypassing CalcHitChance AND the accuracy comparison entirely. This makes OHKO moves (Fissure, Guillotine, Horn Drill) guaranteed to hit when combined with X Accuracy.

**Why not fixed**: This is intentional Gen 1 behavior, not a bug. It was changed in Gen 2+ where X Accuracy boosts accuracy stages instead. Changing it would significantly alter competitive balance.

### Enemy stat-down moves have 25% artificial miss chance

**File**: `engine/battle/effects.asm` (around line 586)

Enemy stat-lowering moves (Growl, Tail Whip, Leer, etc.) have a hardcoded 25% chance to miss in non-link battles, independent of accuracy. This only affects enemy (AI) moves.

**Why not fixed**: This was an intentional difficulty reduction for single-player, not a bug.

### Multi-hit moves only check accuracy once

**File**: `engine/battle/core.asm` (around line 3425)

Multi-hit moves (Double Kick, Fury Attack, etc.) only check accuracy for the first hit. Subsequent hits skip MoveHitTest entirely.

**Why not fixed**: This is the intended Gen 1 behavior.

### Trapping move continuations skip accuracy

**File**: `engine/battle/core.asm` (around line 3734)

Continuation turns of Wrap/Bind/Fire Spin/Clamp skip MoveHitTest. Once the initial hit connects, subsequent turns always hit.

**Why not fixed**: This is the intended Gen 1 behavior.

### Drain/Dream Eater vs Substitute check fixed

**File**: `engine/battle/core.asm` (MoveHitTest, after `.swiftCheck`)

The Swift fix introduced a bug: `CheckTargetSubstitute` overwrites register A with `hWhoseTurn` (0 or 1), so the subsequent `cp DRAIN_HP_EFFECT` / `cp DREAM_EATER_EFFECT` comparisons can never match. HP-draining moves (Leech Life, Mega Drain, etc.) and Dream Eater incorrectly work through Substitutes.

**Fix**: Added `ld a, [de]` (1 byte) after the `call CheckTargetSubstitute` to reload the move effect before the comparisons.

**Tests**: 6 tests in `tests/tests/drain_substitute.rs` covering Drain and Dream Eater hitting/missing against Substitutes, normal moves continuing, and Swift returning immediately.

**Reference**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Substitute_HP_drain_bug)
### PP restoring items fixed to account for PP Ups

**File**: `engine/items/item_effects.asm` (`.fullyRestorePP`)

Max Ethers and Max Elixirs use `.fullyRestorePP` to check if a move already has full PP. The code loads the raw PP byte — which stores PP Up count in the upper 2 bits and current PP in the lower 6 — and compares it directly to the max PP value. With any PP Ups used, the upper bits cause the comparison to fail, so the item is consumed even though PP is already full.

**Fix**: Added `and PP_MASK` (2 bytes) after `ld a, [hl]` to mask out the PP Up bits before comparing. The regular Ether path already does this correctly.

**Tests**: 6 tests in `tests/tests/pp_restore.rs` covering full PP with 0/1/2/3 PP Ups (no effect), and partial/empty PP with PP Ups (restores correctly).


### Counter glitches

**Files**: `engine/battle/core.asm` (`HandleCounterMove`, `EnemySendOutFirstMon`, `SendOutMon`, `CheckPlayerStatusConditions`, `CheckEnemyStatusConditions`), `engine/battle/init_battle_variables.asm`

Three bugs fixed:

1. **Stale damage from switch/battle init**: `wDamage` was never cleared between battles or when Pokemon switch in. Counter could deal damage based on stale data from a previous battle or a switched-out opponent.

   **Fix**: Clear `wDamage` at three points: `InitBattleVariables` (+5 bytes in bank $3D), `EnemySendOutFirstMon` (+5 bytes in bank $0F), and `SendOutMon` (+5 bytes in bank $0F).

2. **Own-damage reflection**: `wDamage` is shared by both sides. When the Counter target can't move (frozen, asleep, fully paralyzed, confusion self-hit), `wDamage` retains stale or self-inflicted damage from the Counter user's own attack, allowing Counter to reflect the user's own damage. This matches the behavior fixed in Pokemon Stadium.

   **Fix**: Clear `wDamage` in the can't-move paths on both player and enemy sides: sleep (`.sleepDone`), frozen (`.FrozenCheck`/`.checkIfFrozen`), and full paralysis/confusion (`.MonHurtItselfOrFullyParalysed`/`.monHurtItselfOrFullyParalysed`). +6 bytes per sleep/frozen path, +7 bytes per paralysis/confusion path = +26 bytes per side, +52 bytes total in bank $0F.

3. **Link battle desynchronization**: `HandleCounterMove` checked `wPlayerSelectedMove`/`wEnemySelectedMove` to determine the target's last move type. But `wPlayerSelectedMove` is updated by cursor movement in the move selection menu (`PrintMenuItem`), not just by actual move confirmation. In link battles, the other Game Boy doesn't see cursor movements — it only knows the actually executed move. If the player moved the cursor to a Normal-type move then switched or used an item, Counter's type check could produce different results on each Game Boy, causing desync.

   **Fix**: Changed `HandleCounterMove` to read the target's move from `wEnemyUsedMove`/`wPlayerUsedMove` (set only when "[Mon] used [Move]!" prints) instead of `wEnemySelectedMove`/`wPlayerSelectedMove` (polluted by cursor movement). Zero-byte change (just different WRAM addresses). Both Game Boys agree on `wUsedMove` since it tracks actually executed moves, which are synchronized via the link exchange.

**Tests**: 18 tests in `tests/tests/counter.rs` — 10 for HandleCounterMove logic (zero damage miss, doubling, overflow cap, full path for Normal/Fighting/Fire types, post-switch clearing, Counter-vs-Counter, zero-power) + 6 ROM byte tests verifying `ld [wDamage], a` in sleep, frozen, and paralysis/confusion paths + 2 ROM byte tests verifying HandleCounterMove loads `wEnemyUsedMove`/`wPlayerUsedMove` (not `SelectedMove`).

**Reference**: [Bulbapedia — Counter glitches](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Counter_glitches) | [Glitch City Wiki — Counter glitches](https://glitchcity.wiki/wiki/Counter_glitches_(Generation_I))
### Bide errors

**File**: `engine/battle/core.asm` (`FaintEnemyPokemon.wild`, `CheckPlayerStatusConditions`, `CheckEnemyStatusConditions`)

Two bugs fixed:

1. **Accumulated damage clearing (link desync)**: When an enemy Pokemon faints, `FaintEnemyPokemon` only cleared the high byte of `wPlayerBideAccumulatedDamage`, leaving the low byte intact (damage became `damage % 256` instead of 0). The counterpart function `RemoveFaintedPlayerMon` correctly clears both bytes. In link battles, the other Game Boy calls `RemoveFaintedPlayerMon` for the same event, causing the two Game Boys to go out of sync unless the accumulated damage was divisible by 256.

   **Fix**: Changed `ld [wPlayerBideAccumulatedDamage], a` to `ld hl, wPlayerBideAccumulatedDamage / ld [hli], a / ld [hl], a` (+2 bytes in bank $0F), matching the pattern in `RemoveFaintedPlayerMon`.

2. **Bide hits through Fly/Dig**: Bide's unleash path skips `MoveHitTest` entirely (jumping straight to `HandleIfPlayerMoveMissed`/`HandleIfEnemyMoveMissed`), which bypasses the INVULNERABLE check that normal moves go through. This allows Bide to always hit opponents in the semi-invulnerable stage of Fly or Dig, revealing their sprite early and causing animation glitches.

   **Fix**: Added an explicit INVULNERABLE bit check before the jump to `HandleIfPlayerMoveMissed`/`HandleIfEnemyMoveMissed`. If the target has INVULNERABLE set, `wMoveMissed` is set to 1. Applied to both player side (`CheckPlayerStatusConditions`) and enemy side (`CheckEnemyStatusConditions`). +12 bytes per side in bank $0F. Fixed in Pokemon Stadium.

**Tests**: 10 tests in `tests/tests/bide.rs` — 4 verifying both bytes of accumulated damage are cleared ($0350, $FFFF, $0080, $0000), and 6 verifying the invulnerability check (player/enemy Bide misses invulnerable target, hits vulnerable target, ignores other status bits).

**Reference**: [Bulbapedia — Bide errors](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Bide_errors) | [Glitch City Wiki — Bide errors](https://glitchcity.wiki/wiki/Bide_errors)

### Defrost move forcing

**File**: `engine/battle/effects.asm` (`CheckDefrost`)

When a frozen Pokémon is defrosted by the opponent's Fire-type move mid-turn, `CheckDefrost` clears the FRZ status bit but doesn't prevent the defrosted Pokémon from attacking. Since the Pokémon was frozen when move selection occurred, `wPlayerSelectedMove`/`wEnemySelectedMove` contains a stale value — the last move the cursor was over, or a move from a different party Pokémon. This causes three problems:

1. **Link desync**: On the owner's Game Boy, the move used is determined by the cursor position in the move menu. On the opponent's Game Boy, it's the last move used (or first move slot). Different values → desync.
2. **PP underflow**: PP is deducted from whatever move executes, regardless of current PP. If the move had 0 PP, it underflows to 63 PP and removes one PP Up.
3. **Wrong move**: The Pokémon uses a move it didn't select (or the glitch move `--` if no move was ever cursor-selected that battle).

**Our fix**: In `CheckDefrost`, after clearing the freeze status, set `wEnemySelectedMove = CANNOT_MOVE` (player path) or `wPlayerSelectedMove = CANNOT_MOVE` (opponent path). `CANNOT_MOVE` ($FF) causes `ExecutePlayerMove`/`ExecuteEnemyMove` to skip the turn entirely. Uses `dec a` (a=0→$FF) for byte efficiency. +8 bytes in bank $0F (Battle Core ROMX).

**Tests**: 9 tests in `tests/tests/defrost_move.rs` — bank $0F check, CheckDefrost entry tests FRZ bit, player path has `dec a / ld [wEnemySelectedMove], a`, opponent path has `dec a / ld [wPlayerSelectedMove], a`, both paths have `xor a` before `dec a` (proves a=0), `.common` is `jp PrintText`, both paths still clear status bytes.

**Reference**: [Bulbapedia — Defrost move forcing](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Defrost_move_forcing) | [Glitch City Wiki — Freeze Desync Glitch](https://glitchcity.wiki/wiki/Freeze_Desync_Glitch)

### Division by 0 (damage calculation freeze)

**File**: `engine/battle/core.asm` (`GetDamageVarsForPlayerAttack`, `GetDamageVarsForEnemyAttack`)

When the attacker's Attack/Special stat exceeds 255, `GetDamageVarsForPlayerAttack`/`GetDamageVarsForEnemyAttack` right-shift both offensive and defensive stats by 2 bits (divide by 4) to fit them in 8-bit registers `b` and `c`. The code correctly clamps the offensive stat to minimum 1 if it becomes 0, but does NOT clamp the defensive stat. If Defense < 4, the shift makes `c = 0`, and `CalculateDamage` divides by `c`, causing an infinite loop freeze.

The same crash occurs when Reflect or Light Screen doubles a defense stat of 512 or 513 to 1024–1026. After the >>2 scaling, `c` becomes 0 (e.g., 1024/4 = 256 = $0100, only the low byte $00 is used as the divisor). Defense stats of 514+ with Reflect/Light Screen don't crash but suffer from rollover — the high byte is silently discarded, making the defense much lower than intended.

**Our fix**: After the >>2 defense shift, add `ld a, c / and a / jr nz, .defNonZero / inc c` to clamp defense to minimum 1. This mirrors the existing attack clamp and the `EXPLODE_EFFECT` defense clamp already in `CalculateDamage`. +5 bytes per path, +10 bytes total in bank $0F.

**Tests**: 9 tests in `tests/tests/division_zero.rs` — bank $0F check, player/enemy paths have `ld a,c / and a / jr nz / inc c` defense clamp between `.scaleStats` and `.next`, clamp is after `srl b` defense shifts, clamp is before `srl h` attack shifts, `.defNonZero` labels exist, EXPLODE_EFFECT defense clamp preserved.

**Reference**: [Bulbapedia — Division by 0](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Division_by_0)

### Experience underflow (Medium Slow level 1 → 100 jump)

**File**: `engine/pokemon/experience.asm` (`CalcExperience`)

The Medium Slow growth rate formula is `(6/5)n³ − 15n² + 100n − 140`. At level 1 (n=1), this evaluates to −54 in integer math (`1 − 15 + 100 − 140 = −54`). Since experience is stored as an unsigned 24-bit value, −54 wraps to $FFFFCA (~16.7 million). When a level 1 Medium Slow Pokémon is added to the party (caught, received as gift, etc.), `CalcExperience(1)` stores this huge wrapped value as the Pokémon's experience. The next time `CalcLevelFromExperience` runs, it determines the Pokémon should be level 100 based on this value far exceeding the max experience for any level.

**Our fix**: At the end of `CalcExperience`, after the final result is written to `hExperience`, check bit 7 of the high byte. Legitimate experience values never exceed ~1.25M ($1312D0), so bit 7 set indicates underflow. If detected, clamp all 3 bytes to 0. +10 bytes in bank $16.

**Tests**: 8 tests in `tests/tests/exp_underflow.rs` — bank $16 check, `bit 7, a` after `.addCubedTerm`, `ret z` follows, `xor a` follows, clamp writes all three hExperience bytes ($96/$97/$98), clamp ends with `ret`, original `ldh [hExperience], a` preserved, `CalcDSquared` follows.

**Reference**: [Bulbapedia — Experience underflow glitch](https://bulbapedia.bulbagarden.net/wiki/Experience#Experience_underflow_glitch) | [Glitch City Wiki — Experience underflow glitch](https://glitchcity.wiki/wiki/Experience_underflow_glitch)

### Experience PC withdrawal freeze (CalcLevelFromExperience MAX_LEVEL cap)

**File**: `engine/pokemon/experience.asm` (`CalcLevelFromExperience`)

When withdrawing a Pokémon from the PC, the game calls `CalcLevelFromExperience` to recalculate its level from stored experience. This routine loops from level 2 upward, calling `CalcExperience(d)` for each level `d` and comparing the result against the Pokémon's stored experience. The loop exits when `exp_needed(d) > current_exp`. However, the register `d` (an 8-bit value) has no upper bound check — if the stored experience exceeds all valid level requirements, `d` wraps past 255→0 and the loop runs forever, softlocking the game.

This is triggered by the experience underflow bug: a level 1 Medium Slow Pokémon's experience wraps to ~16.7M unsigned, which exceeds every level's requirement. While the `CalcExperience` clamp fix (see above) prevents NEW underflows, save data from unpatched games may still contain corrupted experience values. Glitch Pokémon with invalid growth rates or corrupted experience data can also trigger this freeze.

**Our fix**: At the top of `.loop`, after `inc d`, insert `ld a, d / cp MAX_LEVEL + 1 / jr z, .done`. When `d` reaches 101 (MAX_LEVEL + 1), the code falls through to the existing `.done: dec d / ret` path, returning MAX_LEVEL (100) instead of looping forever. The `.done` label is shared with the normal exit path. +5 bytes in bank $16.

**Tests**: 5 tests in `tests/tests/exp_pc_withdraw.rs` — bank $16 check, `.loop` has `inc d / ld a,d / cp 101 / jr z` sequence, `jr z` targets `.done` label, `.done` has `dec d / ret`, `.done` is shared with normal exit (`jr nc .loop` immediately before `.done`).

**Reference**: [Bulbapedia — Experience underflow glitch](https://bulbapedia.bulbagarden.net/wiki/Experience#Experience_underflow_glitch) | [Glitch City Wiki — Experience underflow glitch (Withdrawal lockup)](https://glitchcity.wiki/wiki/Experience_underflow_glitch)

### Hyper Beam + Sleep move glitch

**File**: `engine/battle/effects.asm` (`SleepEffect`)

When a Pokémon is recharging from Hyper Beam and targeted by a sleep-inducing move, `SleepEffect` checked `NEEDS_TO_RECHARGE` in `wXBattleStatus2` and, if set, jumped directly to `.setSleepCounter` — skipping all accuracy checks (`MoveHitTest`), existing status checks (PAR/BRN/PSN/FRZ), and the "already asleep" check. This meant sleep moves always hit recharging targets, overwrote any existing status, and didn't reset the Toxic counter (`BADLY_POISONED` persisted through the forced sleep).

**Our fix**: Removed the `bit NEEDS_TO_RECHARGE, a` test and `jr nz, .setSleepCounter` bypass. The `res NEEDS_TO_RECHARGE, a` is kept so recharge is still cleared, but the code now falls through to the normal accuracy and status checks. −4 bytes in bank $0F.

**Tests**: 8 tests in `tests/tests/hyper_beam_sleep.rs` — bank $0F check, no `bit 5, a` in SleepEffect (removed), no `jr nz` to `.setSleepCounter` bypass, `res 5, a` preserved, `ld [bc], a` follows `res`, `ld a, [de]` follows (normal status path), `call MoveHitTest` preserved, `.sleepEffect` starts with `ld a, [bc]`.

**Reference**: [Bulbapedia — Hyper Beam + Sleep move glitch](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Hyper_Beam_%2B_Sleep_move_glitch) | [Glitch City Wiki — Hyper Beam sleep move glitch](https://glitchcity.wiki/wiki/Hyper_Beam_sleep_move_glitch)

### Index #000 post-capture (battle continues after catching 'M)

**File**: `engine/items/item_effects.asm` (`ItemUseBall`)

When a Pokémon with species index #000 ('M (00) or 3TrainerPoké $) is caught, `ItemUseBall` stores the species index into `wCapturedMonSpecies`. The battle loop at `UseBagItem.checkIfMonCaptured` checks `ld a, [wCapturedMonSpecies] / and a / jr nz` to detect a capture — but species #000 produces `a = 0`, which is indistinguishable from "no capture" (the variable is initialized to 0 at the start of ball use). The battle continues as if nothing was caught. Because `ItemUseBall` also sets the `TRANSFORMED` bit during capture, a subsequent ball throw (in vanilla code) forces `wEnemyMonSpecies2 = DITTO`, producing an invisible wild Ditto.

**Our fix**: Reorder the three stores so `wCurPartySpecies` and `wPokedexNum` (which need the real species index) are written first, then insert `or 1` before storing to `wCapturedMonSpecies`. This guarantees the capture flag is non-zero for any species index, including #000. +2 bytes in bank $03.

**Tests**: 7 tests in `tests/tests/index_000_capture.rs` — bank $03 check, `or 1 / ld [wCapturedMonSpecies], a` present, `wCurPartySpecies` stored before `or 1`, `wPokedexNum` stored before `or 1`, `ld a, [wEnemyMonSpecies]` before stores, battle loop `and a / jr nz` check preserved, `wCapturedMonSpecies` cleared after capture.

**Reference**: [Bulbapedia — Index #000 post-capture](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Index_%23000_post-capture) | [Glitch City Wiki — 'M (00)](https://glitchcity.wiki/wiki/%27M_(00))

### Jump Kick / Hi Jump Kick crash damage (always 1 HP instead of damage/8)

**File**: `engine/battle/core.asm` (`MoveHitTest.moveMissed`, `PrintMoveFailureText`)

When Jump Kick or Hi Jump Kick misses, the crash damage handler divides `wDamage` by 8 and clamps to minimum 1. However, `MoveHitTest.moveMissed` zeroes `wDamage` before the crash handler runs, so the division always computes `max(0/8, 1) = 1` HP of crash damage regardless of the move's actual power. In Generation II, crash damage is correctly 1/8 of the damage the move would have dealt.

**Our fix**: In `MoveHitTest.moveMissed`, save `wDamage` to a new WRAM variable `wJumpKickMissDamage` before zeroing. The crash handler reads from `wJumpKickMissDamage` instead of `wDamage`, divides by 8, clamps to minimum 1, and writes the result to `wDamage` for `ApplyDamageToPlayerPokemon`/`ApplyDamageToEnemyPokemon`. +8 bytes in bank $0F (save), +2 bytes (crash handler redirect), +2 bytes WRAM.

**Tests**: 8 tests in `tests/tests/jump_kick_crash.rs` — bank $0F check, `.moveMissed` saves both bytes to `wJumpKickMissDamage`, `.moveMissed` still zeros `wDamage`, crash handler reads `wJumpKickMissDamage`, crash handler writes to `wDamage`, three `srl a / rr b` shifts (÷8), `or b / jr nz` min clamp, `.enemyTurn` jp preserved.

**Reference**: [Bulbapedia — Jump Kick and Hi Jump Kick's crash damage](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Jump_Kick_and_Hi_Jump_Kick's_crash_damage)

### Level-up learnset skipping (moves missed when skipping levels)

**File**: `engine/pokemon/evos_moves.asm` (`LearnMoveFromLevelUp`)

When a Pokémon gains enough EXP from a single battle to skip one or more levels, `LearnMoveFromLevelUp` only checks for moves at the exact new level. Moves at intermediate levels are silently skipped. For example, a level 4 Pidgey that gains enough EXP to reach level 7 will not learn Sand-Attack (level 5).

**Our fix**: Change `jr nz` (exact level match) to `jr c` (skip only if current level < learn level), so all moves at or below the current level are considered. Save/restore the ROM learnset pointer (`push hl` / `pop hl`) around the party move check and `LearnMove` call so iteration continues through the full learnset instead of returning after the first matching entry. Moves already known are still skipped (existing `.checkCurrentMovesLoop` check). +4 bytes in bank $0E.

**Tests**: 7 tests in `tests/tests/learnset_skipping.rs` — bank $0E check, `jr c` opcode at `.learnSetLoop+10` (not `jr nz`), `push hl` saves learnset pointer, `.continueLearnset` has `pop hl` + `jr .learnSetLoop`, already-known jump targets `.continueLearnset` not `.done`, `.done` is 3 bytes after `.continueLearnset`, `cp b` level comparison present.

**Reference**: [Bulbapedia — Level-up learnset skipping](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Level-up_learnset_skipping)

### Mimic level-up glitch (Mimic'd move reverts on learning new move)

**File**: `engine/pokemon/learn_move.asm` (`DontAbandonLearning`)

When a Pokémon that used Mimic levels up and learns a new move, the `LearnMove` function copies all 4 party moves to battle moves. Since Mimic only modifies battle data (replacing MIMIC with the copied move), the party still has MIMIC in that slot. The bulk copy overwrites the Mimic'd move, reverting it to MIMIC.

**Our fix**: Replace the unconditional 4-byte `CopyData` for moves with a selective loop. For each slot, check if party has MIMIC but battle does not — if so, Mimic was used on that slot, skip the copy to preserve the Mimic'd move. All other slots (normal, newly-learned, unused) are copied normally. PP copy remains unconditional (harmless for non-Mimic'd slots). +11 bytes in bank $01.

**Tests**: 10 tests in `tests/tests/mimic_levelup.rs` — bank $01 check, `.copyMoveLoop` exists with `ld a, [de]`, first `cp MIMIC` checks battle move, `ld a, [hli]` reads party move, `jr z` to `.copyThisMove` when battle=MIMIC, second `cp MIMIC` checks party move, `jr z` to `.skipThisMove` when Mimic active, `ld [de], a` at `.copyThisMove`, `.skipThisMove` has `inc de / dec b / jr nz`, `.copyThisMove` is 1 byte before `.skipThisMove`.

**Reference**: [Bulbapedia — Mimic level up glitch](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Mimic_level_up_glitch)

### Mirror Move link battle desync with trapping moves

**File**: `engine/battle/core.asm` (`MainInBattleLoop`)

In a link battle, if Mirror Move copies a trapping move (Wrap, Fire Spin, etc.) and the opponent switches during the trapping move's continuation, the code checked if the original move was Metronome (which could have randomly selected a trapping move) but not Mirror Move. Without the Mirror Move check, one console interprets the move as Mirror Move while the other sees the trapping move, desynchronizing the battle.

**Our fix**: Add `cp MIRROR_MOVE / jr nz, .specialMoveNotUsed` alongside the existing `cp METRONOME` check in the link battle enemy switch handler. Both Metronome and Mirror Move now jump to `.setSpecialMove` to restore the original move in `wPlayerSelectedMove`. +4 bytes in bank $0F.

**Tests**: 7 tests in `tests/tests/mirror_move_desync.rs` — bank $0F check, `.setSpecialMove` label exists, `cp METRONOME` + `jr z` targets `.setSpecialMove`, `cp MIRROR_MOVE` + `jr nz` targets `.specialMoveNotUsed`, `.setSpecialMove` has `ld [nn], a`, `.specialMoveNotUsed` is 3 bytes after `.setSpecialMove`, `ld a, [hl]` loads move before checks.

**Reference**: [Bulbapedia — Mirror Move glitch](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Mirror_Move_glitch) | [Glitch City Wiki — Partial trapping move Mirror Move link battle glitch](https://glitchcity.wiki/wiki/Partial_trapping_move_Mirror_Move_link_battle_glitch)

### 0 damage glitch (0.25x effective move misses instead of dealing 1)

**File**: `engine/battle/core.asm` (`AdjustDamageForMoveType`)

The minimum damage before type effectiveness is 2 (3 with STAB). When a move is 0.25x effective (dual-type double resistance), `floor(2/4) = floor(3/4) = 0`. The code treated 0 damage identically to type immunity, setting `wMoveMissed = 1` and displaying "Attack missed!" — even though the move should connect for at least 1 damage.

**Our fix**: After finding 0 damage, check `wDamageMultipliers & EFFECTIVENESS_MASK`. If zero, the target is truly immune — set `wMoveMissed` as before. If non-zero, the move connected but rounded to 0 — clamp damage to 1 (`ld [hl], 1`) instead of missing. +11 bytes in bank $0F.

**Tests**: 8 tests in `tests/tests/zero_damage.rs` — bank $0F check, banked range, `ld a, [wDamageMultipliers]` in 0-damage path, `and EFFECTIVENESS_MASK` follows, `jr z` to immunity path, `ld [hl], 1` clamp present, `.typeImmunity` sets `wMoveMissed`, clamp before immunity ordering.

**Reference**: [Bulbapedia — 0 damage glitch](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#0_damage_glitch) | [Glitch City Wiki — 0 damage miss glitch](https://glitchcity.wiki/wiki/0_damage_miss_glitch)

### AI trainer HUD does not update when it uses healing items

**File**: `engine/battle/trainer_ai.asm` (`AIPrintItemUseAndUpdateHPBar`, `AIUseFullHeal`)

When an AI trainer uses a healing item (Potion, Super Potion, Hyper Potion, Full Restore, Full Heal), the function `DrawEnemyHUDAndHPBar` is never called. The HP bar animation plays via `UpdateHPBar2`, but the full HUD redraw is skipped. This causes two visible issues: (1) status icons don't clear until after the player's turn (Full Heal, Full Restore), and (2) HP bar color doesn't update when HP crosses a color threshold — e.g., the bar stays yellow even though HP was restored into the green zone.

**Fix**: Create shared `DrawEnemyHUDAndDecrementAICount` label containing `callfar DrawEnemyHUDAndHPBar` + `jp DecrementAICount`. `AIPrintItemUseAndUpdateHPBar` (Potion/Super/Hyper/Full Restore) falls through to it; `AIUseFullHeal` (Full Heal) jumps to it after inlining `AIPrintItemUse_`. Total: +14 bytes in bank $0E.

**Tests**: 6 ROM byte tests in `tests/tests/ai_hud.rs` verifying the shared label callfar expansion, fallthrough from potion path, and jp from Full Heal path.

### Lt. Surge gym trash can second lock uses wrong register

**File**: `engine/events/hidden_events/vermilion_gym_trash2.asm` (`TrashCanRandom.three`)

Lt. Surge's gym has a trash can puzzle where the player must find two locks. After finding the first lock, a second lock is randomly placed in an adjacent trash can. The `TrashCanRandom` function selects a random index from the adjacency list using a jump table based on the number of valid neighbor pairs (2, 3, or 4). The `.three` case — used by 8 of the 15 trash cans — returned the result in register `b` instead of `a`. The caller reads from `a`, so the table offset was the raw random value (0–255) instead of the intended [0, 1, 2]. This caused garbage second lock selections — most commonly trash can 0 (top-left) regardless of which can had the first lock.

**Our fix**: Rewrite `.three` to return in register `a` using `jr nc` / `xor a` / `ld a, 1` / `ret c` / `inc a` / `ret`. Same byte count (18 bytes), zero ROM growth.

**Tests**: 7 tests in `tests/tests/surge_trash.rs` — 4 ROM byte verification tests (`jr nc` opcode, `xor a` for zero case, `ld a, 1` / `ret c` / `inc a` for non-zero, `jr nc` offset targeting `.three_not_zero`) and 3 behavioral emulator tests (valid adjacency for 3-entry cans, valid adjacency for all 15 cans, full pair coverage).

**Reference**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Vermilion_Gym#Gym_puzzle)

### Item stack overflow corrupts memory when adding past 99

**File**: `engine/items/inventory.asm` (`AddItemToInventory_`)

When adding items to an existing stack where the total exceeds 99, `AddItemToInventory_` splits the stack: caps the current slot at 99 and tries to place the remainder. After the split, it re-enters the search loop at `.addAnotherStackOfItem` to find or create a slot for the remainder. The search loop reads bytes sequentially, checking each for the `$FF` list terminator. However, it can scan past the terminator into unrelated WRAM, interpreting arbitrary bytes as item ID / quantity pairs and potentially corrupting memory by modifying "quantities" of non-existent items.

**Our fix**: After writing 99 to the current slot, jump directly to `.addNewItem` instead of `.addAnotherStackOfItem`. `.addNewItem` calculates the correct new slot position from `wNumBagItems` and safely creates the remainder entry with a proper terminator. Same byte count (`jp` = 3 bytes), zero ROM growth.

**Note**: The Glitch City Wiki states this bug "does not work in English Yellow" — the code-level bug exists but doesn't manifest because the WRAM layout past the bag buffer doesn't contain matching item IDs. We fix the code anyway to prevent any possible corruption.

**Tests**: 9 tests in `tests/tests/item_overflow.rs` — 1 ROM byte test (jp target is `.addNewItem`), 8 behavioral tests (split creates correct new slot, no-split add, new item type, full bag rejection, capacity boundary, terminator placement, WRAM sentinel integrity, empty bag).

**Reference**: [Glitch City Wiki](https://glitchcity.wiki/wiki/99_item_stack_glitch)

### Route 16 sign unreadable from the front

**File**: `data/maps/objects/Route16.asm`, `data/maps/objects/Route17.asm`, `scripts/Route17.asm`

The "ROUTE 16 / CELADON CITY - FUCHSIA CITY" sign sits at tile coordinate Y=17 — the very last tile row of the Route 16 map (20×9 blocks = 40×18 tiles). When the player stands directly south of the sign to read it, they've crossed the map boundary into Route 17 (connected south with offset 0). The game only checks bg_events for the current map, so Route 16's sign event is never matched while the player is on Route 17.

**Our fix**: Add a duplicate `bg_event 5, -1, TEXT_ROUTE17_ROUTE_16_SIGN` in Route 17's object data. Y=-1 ($FF) corresponds to the connection strip tile one row above Route 17's boundary — exactly where the sign sits. A new text pointer `Route17Route16SignText` in Route 17's script references the same `_Route16SignText` string via `text_far`. Zero ROM growth for text (reuses Route 16's string), +3 bytes for the bg_event, +3 bytes for the text pointer entry, +3 bytes for the text_far label.

**Tests**: 3 tests in `tests/tests/route16_sign.rs` — Route 17 bg_event count and coordinates, Route 16 sign still present, text ID validity.

**References**: [Glitch City Wiki](https://glitchcity.wiki/wiki/Cycling_Road_sign_glitch)

### Invisible tree glitch (cut tree returns as invisible wall)

**File**: `engine/overworld/player_state.asm`, `ram/wram.asm`

After cutting a tree near a map border (e.g. Route 14/15), walking toward the border triggers a map connection. `LoadTileBlockMap` rebuilds `wOverworldMap` from ROM (restoring the tree in the block map), but `wTileMap` and VRAM retain the stale "no tree" tiles from before the connection. This creates a desync: collision detection uses `wOverworldMap` (tree present → blocked), while the screen shows the tile buffer (no tree visible). The player hits an invisible wall they cannot re-cut because `_GetTileAndCoordsInFrontOfPlayer` reads from `wTileMap`, which may report the tree tile `$3D` even though VRAM displays the empty replacement tile.

**Our fix**: At `_GetTileAndCoordsInFrontOfPlayer.storeTile`, if `wTileMap` reports `$3D` (OVERWORLD cut tree tile), verify against VRAM via `ReadTileFromVram`. This new subroutine computes the actual VRAM tile address using `rSCX`/`rSCY` scroll offsets and the BG tile map at `$9800`, then polls until the tile is accessible (not `$FF` during LCD mode 3). If VRAM has a different tile (tree hasn't been visually redrawn yet), the VRAM value is used instead. Screen coordinates are stored in `wTempColCoords` (2 bytes, overlaid in existing NEXTU block) before each `lda_coord` call. +95 bytes in bank $03.

**Tests**: 11 tests in `tests/tests/invisible_tree.rs` — coordinate storage for all 4 facing directions, `cp $3D` check at `.storeTile`, `call z, ReadTileFromVram` target, `wTileInFrontOfPlayer` store preserved, `ReadTileFromVram` structure (push bc, rSCX/rSCY reads, BG tilemap base `$9800`, ret).

**References**: [Glitch City Wiki](https://glitchcity.wiki/wiki/Invisible_tree_glitch), [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_(Generation_I)#Invisible_tree)

### Ledge jump lands on NPC

**File**: `engine/overworld/ledges.asm` (HandleLedges.foundMatch)

When the player jumps a ledge, `HandleLedges.foundMatch` checks the direction button and immediately starts the jump animation without verifying whether an NPC sprite occupies the landing tile (2 tiles ahead). Normal walking checks sprite collisions, but the ledge code path bypasses this entirely. By luring an NPC below a ledge, the player can land directly on top of the NPC.

**Our fix**: Before committing to the jump, call `IsSpriteInFrontOfPlayer2` with `d = $20` (32 pixels = 2-tile range, matching the ledge jump distance) to check the landing position. If `hSpriteIndex` is nonzero after the call, a sprite occupies the landing tile and the jump is cancelled with `ret nz`. `push de`/`pop de` preserves the direction button mask (`e`) across the check. +14 bytes in bank $06.

**Tests**: 11 tests in `tests/tests/ledge_npc.rs` — joy held check structure, push de before sprite check, xor a clears hSpriteIndex, ld d $20 range, call IsSpriteInFrontOfPlayer2 target, ldh a [hSpriteIndex] read, pop de + ret nz sequence, original code resumes, 14-byte check structure, bank $06 placement, IsSpriteInFrontOfPlayer2 HOME bank.

**References**: [Glitch City Wiki](https://glitchcity.wiki/wiki/NPC_collision_bypassing_glitch), [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_(Generation_I))

### Bicycle music persists through hole warp

**File**: `engine/overworld/player_animations.asm` (LeaveMapThroughHoleAnim)

When the player falls through a hole while riding the Bicycle, the bike music continues playing on the destination map. `LeaveMapThroughHoleAnim` handles the visual falling animation but never resets the music. The player dismounts the bike upon landing (cleared by `LoadPlayerSpriteGraphics` during `LoadMapData`), but `PlayDefaultMusicFadeOutCurrent` sees `wWalkBikeSurfState` already cleared and plays the map music — except the bike music is still fading in from the previous map. This primarily affects Seafoam Islands (CAVERN tileset allows biking).

**Our fix**: At the start of `LeaveMapThroughHoleAnim`, check if `wLastMusicSoundID` equals `MUSIC_BIKE_RIDING`. If so, call `PlayDefaultMusic` to reset the music before the animation plays. This ensures the map music starts playing before the transition. +8 bytes in bank $1C.

**Tests**: 7 tests in `tests/tests/bicycle_hole.rs` — ld a [wLastMusicSoundID] structure, cp MUSIC_BIKE_RIDING ($D2) value, call z PlayDefaultMusic target, original code resumes at offset 8, 8-byte check structure, bank $1C placement, PlayDefaultMusic HOME bank.

**References**: [Glitch City Wiki](https://glitchcity.wiki/wiki/Bicycle_music_hole_glitch), [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_(Generation_I)#Victory_Road_Bicycle_music_quirk)

### Cycling Road access requirement bypassing (no Bicycle needed)

**Files**: `scripts/Route16Gate1F.asm` (`Route16Gate1FDefaultScript`), `scripts/Route18Gate1F.asm` (`Route18Gate1FDefaultScript`)

The Cycling Road gate guards check whether the player has a Bicycle. If not, they show "Excuse me! Wait up!" text and push the player back with simulated PAD_RIGHT movement. However, the player can bypass this by holding LEFT on the d-pad while the guard text displays. The `.next_to_counter` code path (player at y=7, right at the counter) transitions directly to the GUARD script state without calling `StartSimulatingJoypadStates` or setting `wJoyIgnore`, so the real d-pad input passes through and overrides the forced movement. Upon entering Cycling Road, the player is automatically put on a bike despite not having one.

**Our fix**: Set `wJoyIgnore = PAD_CTRL_PAD` immediately after the coordinate check succeeds (`ret nc`) and before the guard text displays. This blocks d-pad input from the moment the guard intercepts the player, preventing the LEFT override for both the `.next_to_counter` and normal walk-up paths. The mask is cleared by `PlayerMovingRightScript` after the forced push-back completes. +5 bytes per gate script, +10 bytes total in bank $12.

**Tests**: 8 tests in `tests/tests/cycling_road_bypass.rs` — bank $12 check (both gates), `ld a, PAD_CTRL_PAD / ld [wJoyIgnore], a` after `ret nc` (both gates), wJoyIgnore store before DisplayTextID (both gates), `xor a / ld [wJoyIgnore], a` in PlayerMovingRightScript to clear mask (both gates).

**References**: [Glitch City Wiki — Go on Cycling Road without a Bicycle](https://glitchcity.wiki/wiki/Go_on_Cycling_Road_without_a_Bicycle) · [Bulbapedia — List of glitches in Generation I](https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_I)

### Escape sprite shows garbled tiles during teleport animation

**File**: `engine/overworld/player_animations.asm` (`PlayerSpinWhileMovingUpOrDown`)

When using Escape Rope, Dig, or Teleport, the player sprite briefly shows garbled "ABCD" tiles (DMG) or doesn't spin correctly (SGB) during the upward movement phase. `PlayerSpinWhileMovingUpOrDown` is entered with `hl` pointing to `wPlayerSpinWhileMovingUpOrDownAnimFrameDelay` instead of `wFacingDirectionList`, so `SpinPlayerSprite` reads the frame delay value (2 or 3) as a sprite facing index, producing incorrect tile data.

**Our fix**: At the top of `PlayerSpinWhileMovingUpOrDown`, add `ld hl, wFacingDirectionList` before calling `SpinPlayerSprite`. This ensures `hl` always points to the correct facing direction buffer regardless of how the caller left it. Fixes all callers (both `.spinWhileMovingUp` and `PlayerSpinWhileMovingDown`). +3 bytes in bank $1C.

**Tests**: 6 tests in `tests/tests/escape_sprite.rs` — `ld hl, wFacingDirectionList` at start, `call SpinPlayerSprite` follows, original code at offset +6, bank $1C placement, same-bank assertion, WRAM address check.

**References**: [Glitch City Wiki](https://glitchcity.wiki/wiki/Escape_sprite_handling_glitch), [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_(Generation_I))

### Strength boulder smoke puffs corrupted by OAM bug

**File**: `engine/battle/animations.asm` (`AdjustOAMBlockYPos2`)

When pushing Strength boulders, dust/smoke puff sprites animate using OAM Y-coordinate adjustments. `AdjustOAMBlockYPos2` checks if the adjusted Y >= 112 (off-screen) and intends to hide the sprite by setting Y to 160. However, the code does `dec hl` before writing — since `hl` points to the Y byte (byte 0 of the OAM entry), `dec hl` backs into the **previous** entry's attribute byte (byte 3). Writing 160 ($A0) there corrupts the previous sprite's palette, flip, and priority flags, causing smoke puffs to display incorrectly when pushing boulders (especially downward).

**Our fix**: Remove the `dec hl / ld a, 160 / ld [hli], a` sequence. Replace with a simple conditional: if Y >= 112, set A = 160 before writing to `[hl]`. The `ld [hl], a` at `.noOverflow` correctly writes either the adjusted Y (if on-screen) or 160 (if off-screen). −2 bytes in bank $1E.

**Tests**: 10 tests in `tests/tests/boulder_smoke.rs` — bank check, `ld de, OBJ_SIZE` at start, `add b` + `cp 112` threshold, `jr c` targets `.noOverflow`, `ld a, 160` before `.noOverflow`, `ld [hl], a` at `.noOverflow`, no `dec hl` ($2B) in conditional block, no `ld [hli], a` ($22) in conditional block, `ret` at end.

**Reference**: [pret/pokered wiki](https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches)

### Battle transition doesn't recognize some dungeon maps

**File**: `data/maps/dungeon_maps.asm` (`DungeonMaps1`, `DungeonMaps2`)

`GetBattleTransitionID_IsDungeonMap` determines whether a map uses dungeon-style or outdoor-style battle transition animations. It checks `wCurMap` against two lists: `DungeonMaps1` (exact matches) and `DungeonMaps2` (range checks). Several obvious dungeon maps were missing from both lists due to non-contiguous map IDs across different map groups: Victory Road 2F/3F, all Rocket Hideout floors (B1F-B4F), Pokémon Mansion 1F, Seafoam Islands B1F-B4F, Power Plant, Diglett's Cave, and Silph Co. 9F-11F.

**Our fix**: Add 5 individual entries to `DungeonMaps1` (Pokémon Mansion 1F, Victory Road 2F/3F, Power Plant, Diglett's Cave) and 3 range entries to `DungeonMaps2` (Silph Co. 9F-11F, Seafoam Islands B1F-B4F, Rocket Hideout B1F-B4F). +11 bytes in bank $1C.

**Tests**: 13 tests in `tests/tests/dungeon_maps.rs` — list sizes (9 entries, 7 ranges), original entries present, each new exact-match entry present, each new range present, integration test verifying all 16 previously-missing maps are now recognized.

**Reference**: [pret/pokered wiki](https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches)

### Slot machine tile loading overrun

**File**: `engine/slots/slot_machine.asm` (`LoadSlotMachineTiles`)

`LoadSlotMachineTiles` loads `$1C tiles` (28 tiles = 448 bytes) of `SlotMachineTiles2` data into VRAM, but the actual tile data (`gfx/slots/slots_2.2bpp`) is only `$18 tiles` (24 tiles = 384 bytes). This copies 64 extra bytes of whatever ROM data follows `SlotMachineTiles2End` into VRAM. The overrun occurs in both copy operations (to `vChars0` and to `vChars2 tile $25`). This doesn't cause visible issues during normal play because those VRAM tile slots are overwritten before use.

**Our fix**: Replace both `$1c tiles` with `SlotMachineTiles2End - SlotMachineTiles2` (the symbolic size calculation already used by `SlotMachineTiles1`). Zero ROM growth — only the immediate operands change.

**Tests**: 8 tests in `tests/tests/slot_tiles.rs` — bank check, tile data size = $180, both `ld bc` operands match actual data size, neither uses old buggy value ($1C0), both loads use same size, `SlotMachineTiles1` size verified.

**Reference**: [pret/pokered wiki](https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches)

### Lucky slot machine doesn't stop wheel on 7

**File**: `engine/slots/slot_machine.asm` (`SlotMachine_StopWheel1Early.sevenAndBarMode`)

In the Game Corner, one slot machine is designated "lucky" and is meant to stop wheel 1 immediately when a 7 symbol appears in the window. `SlotMachine_StopWheel1Early.sevenAndBarMode` loops through the 3 visible tiles, comparing each with `cp HIGH(SLOTS7)` ($02) and then `jr c, .stopWheel`. The `jr c` condition triggers when the carry flag is set (A < $02), but all valid slot symbol HIGH bytes are >= $02 (SLOTS7 = $02 is the minimum). The condition is never true, so the wheel never stops early for 7s — the lucky machine behaves identically to normal machines.

**Our fix**: Change `jr c` ($38) to `jr z` ($28). Now the wheel stops when A == $02 (a 7 symbol). Single-byte fix, zero ROM growth.

**Tests**: 8 tests in `tests/tests/slot_lucky.rs` — bank check, loop counter = 3, tile read via `[hli]`, compare operand = $02, `jr z` opcode (not `jr c`), jump target = `.stopWheel`, wheel 2 threshold verified.

**Reference**: [pret/pokered wiki](https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches)

### Lucky slot machine wheel 2 stops early without a match

**File**: `engine/slots/slot_machine.asm` (`SlotMachine_StopWheel2Early.sevenAndBarMode`)

`SlotMachine_StopWheel2Early.sevenAndBarMode` calls `SlotMachine_FindWheel1Wheel2Matches` to check if wheels 1 and 2 have matching symbols on any payline. When no match is found (NZ), DE is decremented back to `wSlotMachineWheel2BottomTile`. The code then does `ld a, [de]` / `cp HIGH(SLOTSBAR) + 1` / `ret nc` without checking the Z flag, so if the bottom tile of wheel 2 happens to be a 7 or bar, the wheel stops early even though there's no matching symbol on wheel 1. This reduces the player's odds on the lucky machine by freezing wheel 2 in a position that can't produce a winning payline.

**Our fix**: Add `ret nz` after `call SlotMachine_FindWheel1Wheel2Matches` to bail out immediately when no match exists. +1 byte in bank $0D.

**Tests**: 8 tests in `tests/tests/slot_wheel2.rs` — bank check, call opcode, `ret nz` ($C0) at +3, `ld a, [de]` at +4, `cp $07` / `ret nc` threshold, `.stopWheel` offset, `xor a` at `.stopWheel`.

**Reference**: [pret/pokered wiki](https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches)

### Lucky slot machine can be nonexistent slot −1 (off-by-one)

**File**: `scripts/GameCorner.asm` (`GameCornerSelectLuckySlotMachine`)

When the Game Corner map loads, `GameCornerSelectLuckySlotMachine` picks a random byte (0–255), and values below a threshold are replaced with 8 before three right-shifts (`srl a` ×3) produce the slot machine index. The comparison `cp $7` / `jr nc, .not_max` / `ld a, $8` catches values 0–6, but value 7 slips through: 7 >> 3 = 0, an invalid index (slot machines are 1-indexed). This selects the nonexistent "slot machine −1" with a 1/256 (~0.4%) probability.

**Our fix**: Change `cp $7` to `cp $8` so that value 7 is also caught and replaced with 8. One-byte change, zero ROM growth.

**Tests**: 8 tests in `tests/tests/lucky_slot.rs` — bank check, banked range, `cp $8` present (not `cp $7`), `jr nc` follows, `ld a, $8` follows, three `srl a` at `.not_max`, result stored to WRAM, no old `cp $7` pattern.

**Reference**: [Glitch City Wiki — Slot machine behaviors (Generation I)](https://glitchcity.wiki/wiki/Slot_machine_behaviors_(Generation_I)#The_lucky_slot_machine)

### Hidden 40-coin stash gives only 20 coins

**File**: `engine/events/hidden_items.asm` (`HiddenCoins`)

The Celadon Game Corner has a hidden coin stash at coordinates (11, 7) worth 40 coins. `HiddenCoins` subtracts `COIN` from the function argument and compares the result against 10, 20, 40, and 100 to determine the BCD coin value. The `cp 40` / `jr z` on line 85 jumps to `.bcd20` instead of `.bcd40` due to a typo. The `.bcd40` label exists (loads BCD $40) but was unreachable. The player receives 20 coins instead of the intended 40.

**Our fix**: Change `jr z, .bcd20` to `jr z, .bcd40`. Zero ROM growth — only the relative jump offset changes.

**Tests**: 8 tests in `tests/tests/hidden_coins.rs` — bank check, all 4 BCD labels load correct values ($10/$20/$40/$01), `jr z` after `cp 40` targets `.bcd40` not `.bcd20`, `jr z` after `cp 20` still targets `.bcd20`.

**References**: [Glitch City Wiki — Inaccessible coins](https://glitchcity.wiki/wiki/Inaccessible_coins) · [Bulbapedia — List of glitches in Generation I](https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_I)

### Game Corner 10-coin tile oversight (inaccessible hidden coins)

**Files**: `data/events/hidden_coins.asm`, `data/events/hidden_events.asm`

The Celadon Game Corner has hidden coins at tile coordinates (12, 15). However, this tile contains a slot machine being used by an NPC (Gambler sprite at 11, 15 facing right). The player cannot walk onto or interact with the tile to collect the coins, making them permanently inaccessible.

**Our fix**: Remove the `hidden_coin GAME_CORNER, 12, 15` entry from `HiddenCoinCoords` and the corresponding `hidden_event 12, 15, HiddenCoins, COIN+10` from the Game Corner hidden events list. −3 bytes in `HiddenCoinCoords` (data), −4 bytes in hidden events (data).

**Tests**: 5 tests in `tests/tests/game_corner_coins.rs` — bank $1D check, 11 entries (down from 12), no entry at (12,15), other coins preserved (spot check 5 known entries), table $FF terminated.

**References**: [Bulbapedia — List of glitches in Generation I](https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_I) · [Glitch City Wiki — Game Corner 10 coins tile oversight](https://glitchcity.wiki/wiki/Game_Corner_10_coins_tile_oversight)

### Splash screen adds 2 extra invisible stars

**File**: `engine/movie/splash.asm` (`AnimateShootingStar`)

The Game Freak splash screen animates small stars falling from the logo in 4 waves of 4 OAM entries each. After placing each wave, `wMoveDownSmallStarsOAMCount` is incremented by 6 (`add 6`) instead of 4. The extra 2 entries per wave reference OAM slots that were initialized off-screen by the `initSmallStarsOAMLoop`, so they are invisible — but they waste OAM entries and CPU cycles in `MoveDownSmallStars`, which iterates over `wMoveDownSmallStarsOAMCount` entries each frame.

**Our fix**: Change `add 6` to `add 4`. Zero ROM growth — only the immediate operand changes.

**Tests**: 8 tests in `tests/tests/splash_stars.rs` — bank check, `add` operand is 4 not 6, `cp 24` OAM count cap, inner loop counter is 4, outer loop counter is 6, init OAM count is 24, wave pointer table structure (4 real + 2 empty).

**Reference**: [pret/pokered wiki](https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches)

### Healing machine PC screen tile loading overrun

**File**: `engine/overworld/healing_machine.asm` (`AnimateHealingMachine`)

`AnimateHealingMachine` copies tile data from `PokeCenterFlashingMonitorAndHealBall` (the monitor and heal ball graphics) into VRAM. The `CopyVideoData` call specifies 3 tiles, but `gfx/overworld/heal_machine.2bpp` only contains 2 tiles (32 bytes: 1 monitor tile + 1 heal ball tile). The third "tile" copies 16 bytes of garbage from `PokeCenterOAMData` that follows immediately in ROM, overwriting VRAM tile $7E with OAM coordinate data interpreted as pixel data.

**Our fix**: Change the tile count from 3 to 2. Zero ROM growth — only the immediate operand changes.

**Tests**: 8 tests in `tests/tests/healing_machine.rs` — bank check, tile data size is 32 bytes (2 tiles), `ld bc` tile count operand is 2 not 3, `ld de` points to tile data, bank byte matches, monitor sprite uses tile $7C, heal ball uses $7D.

**Reference**: [pret/pokered wiki](https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches)

### GetName TM/HM redirect applies to all name types

**File**: `home/names2.asm` (`GetName`)

`GetName` is a general-purpose name lookup function used for Pokémon, moves, items, trainers, and OT names. It checks `wNameListType` to determine which name table to use. However, before that dispatch, it unconditionally compares the name index against `HM01` ($C4) and redirects to `GetMachineName` if the index is >= $C4. This TM/HM redirect should only apply when looking up item names (`ITEM_NAME`), but it runs for all name types. In vanilla, the bug is latent because `NUM_POKEMON_INDEXES`, `NUM_ATTACKS`, and `NUM_TRAINERS` are all < HM01, so no valid non-item index can trigger it.

**Our fix**: Check `wNameListType == ITEM_NAME` before the `cp HM01` comparison. SM83 `ld` instructions don't affect flags, so the Z flag from `cp ITEM_NAME` survives through the subsequent `ld a, [wNameListIndex]` / `ld [wNamedObjectIndex], a`, allowing a compact `jr nz, .notMachine` gate. +7 bytes in HOME (ROM0).

**Tests**: 8 tests in `tests/tests/getname.rs` — bank 0 check, first instruction loads `wNameListType`, `cp ITEM_NAME` ($04) follows, `jr nz` targets `.notMachine`, `cp HM01` gated by item check, `jp nc` targets `GetMachineName`, name index stored to `wNamedObjectIndex`, no unconditional HM01 check at start.

**Reference**: [pret/pokered wiki](https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches)

### Cycling Road flags leak into new game

**File**: `engine/movie/oak_speech/oak_speech.asm` (`PrepareOakSpeech`)

`PrepareOakSpeech` saves `wStatusFlags6` before clearing memory (to preserve `BIT_DEBUG_MODE` for `StartNewGameDebug`), then restores it unmodified afterward. If the player's previous save was at the Cycling Road, `BIT_ALWAYS_ON_BIKE` (bit 5) is set in `wStatusFlags6`. This bit carries over into the new game, causing `CheckForceBikeOrSurf` to return immediately without clearing it — the player is stuck on the bike in Pallet Town. Other stale flags (`BIT_FLY_OR_DUNGEON_WARP`, `BIT_ESCAPE_WARP`, etc.) also leak through.

**Our fix**: Wrap the `wStatusFlags6` save/restore in `IF DEF(_DEBUG)` / `ENDC` so it's only present in debug builds. In release builds, `FillMemory` clears `wStatusFlags6` along with the rest of WRAM, ensuring all stale flags are cleared. Saves 8 bytes in bank $01 for release builds vs the original code, and matches the existing conditional compilation pattern for debug features.

**Tests**: 8 tests in `tests/tests/newgame_flags.rs` — bank check, no `ld a, [wStatusFlags6]` in release, no `ld [wStatusFlags6], a` in release, wOptions still saved/restored, no unmasked `pop af / ld [wStatusFlags6]` pattern, no `and $02` mask pattern, BIT_ALWAYS_ON_BIKE logic check.

**References**: [Bulbapedia — List of overworld glitches (Generation I)](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_(Generation_I)) · [Glitch City Wiki — Ghost Bicycle glitch](https://glitchcity.wiki/wiki/WRAM_clear_oversight_(Generation_I))

### ED tile not displayed correctly on bad emulators

**File**: `engine/menus/naming_screen.asm` (`LoadEDTile`)

In Red/Blue, the ED tile (used on the naming screen) specified bank 0 for its graphics data. MBC3 cartridges mapped bank 0 writes as bank 1, so it worked. Pokemon Yellow uses MBC5, which correctly maps bank 0 as bank 0 — so the tile would load from the wrong address. GameFreak worked around this by embedding the tile data directly in `LoadEDTile` and manually copying it to VRAM during HBlank. This manual HBlank copy works on real hardware and accurate emulators, but fails on poorly-coded emulators that don't properly implement HBlank timing, causing garbled graphics on the naming screen.

**Our fix**: Replace the manual HBlank copy loop with `jp CopyVideoDataDouble`, which uses proper V-blank DMA timing and ROM banking. This is compatible with all emulators while being cleaner and smaller code (-14 bytes in bank $01).

**Tests**: 8 tests in `tests/tests/ed_tile.rs` — bank check, ED tile data is 8 bytes, `jp CopyVideoDataDouble` present, tile count is 1, bank operand matches ED_Tile, `ld de` points to ED_Tile, no HBlank STAT polling, function is 12 bytes.

**Reference**: [Bulbapedia — Character encoding (Generation I)](https://bulbapedia.bulbagarden.net/wiki/Character_encoding_(Generation_I))

### Item Finder fails to detect items at coordinate 0

**File**: `engine/items/itemfinder.asm` (`HiddenItemNear`)

The Item Finder's proximity check uses `Sub5ClampTo0` to compute a lower bound (`max(playerCoord - 5, 0)`) and then `cp d / jr nc, .loop` to skip items whose coordinate is below that bound. The `jr nc` condition triggers when the carry flag is clear, which happens when A >= D. But equality (A == D) means the item is at the exact detection boundary — it should be found, not skipped. When the lower bound is 0 (player at coordinates 0–5) and the item is at coordinate 0, the comparison is 0 == 0 → carry clear → item skipped.

No vanilla Yellow hidden items are placed at X=0 or Y=0, so this bug is latent in the base game. It matters for ROM hacks that place items at coordinate 0.

**Our fix**: Add `jr z, .checkYUpper` / `jr z, .checkXUpper` before each `jr nc, .loop` so that equality is treated as "in range". +4 bytes in bank $1D.

**Tests**: 8 tests in `tests/tests/itemfinder.rs` — `jr z` opcodes and offsets for both Y and X lower bounds, `Sub5ClampTo0` structure, upper bound `add 4`/`add 5` preserved, bank $1D check.

### NPC overworld movement not restricted correctly

**File**: `engine/overworld/movement.asm` (`CanWalkOntoTile`)

NPC sprites use displacement counters (`SPRITESTATEDATA2_YDISPLACEMENT` and `XDISPLACEMENT`, both initialized to `$8`) to track how far they've walked from their home position. The upward and leftward checks correctly block movement when the counter reaches 0 (`sub $1 / jr c, .impassable`), limiting NPCs to 8 steps in those directions. However, the downward and rightward checks have `cp $5` comparisons with no conditional jump — the code unconditionally falls through to the next section, making the upper bound a no-op. NPCs can walk unlimited steps down or right until the displacement counter overflows at 255.

In Red/Blue, the downward check had `jr c, .impassable` after `cp $5`, but this caused NPCs to get stuck after walking 5 steps upward (the downward return path was blocked). Yellow commented out this check as a partial fix, but left both down and right unlimited.

**Our fix**: Replace the dead `cp $5` with `cp $11 / jr nc, .impassable` for both down and right paths. This blocks movement when displacement reaches `$11` (17), creating a symmetric 8-step bound in all 4 directions (range `$0`–`$10`, centered at `$8`). +4 bytes in bank $01.

**Tests**: 8 tests in `tests/tests/npc_movement.rs` — `cp $11` and `jr nc, .impassable` for both Y and X upper bounds, upward/leftward lower bounds preserved (`sub $1 / jr c`), displacement initialization at `$8`, bank $01 check.

**Reference**: [Glitch City Wiki](https://glitchcity.wiki/wiki/NPC_walking_behavior_glitches)

### NPC offscreen detection off-by-one (bottom row / rightmost column)

**File**: `engine/overworld/movement.asm` (`CanWalkOntoTile`)

The screen boundary checks use `cp $80` for Y and `cp $90` for X with `jr nc, .impassable`. Since `jr nc` triggers when A >= the operand, pixel positions `$80` (128, the bottom row) and `$90` (144, the rightmost column) are incorrectly treated as offscreen. NPCs on those map borders cannot move horizontally (bottom row) or vertically (rightmost column).

**Our fix**: Change `cp $80` to `cp $81` and `cp $90` to `cp $91`. Zero ROM growth — only the immediate operands change.

**Tests**: 2 tests in `tests/tests/npc_movement.rs` — `cp $81` after `add d` for Y boundary, `cp $91` after `add e` for X boundary.

**Reference**: [Glitch City Wiki](https://glitchcity.wiki/wiki/NPC_walking_behavior_glitches#Glitch_related_to_the_border_of_the_screen)

### NPC movement delay wraparound

**File**: `engine/overworld/movement.asm` (`UpdateSpriteMovementDelay`)

After an NPC finishes walking, a random delay in [0, $7F] is generated before the next movement. `UpdateSpriteMovementDelay` decrements this counter with `dec [hl]` and checks `jr nz` to see if it has reached 0. When the random delay is 0, `dec` wraps from 0 to $FF (255), and the NPC waits an additional 256 frames (~4.3 seconds) before moving again.

**Our fix**: Add `ld a, [hl] / and a / jr z, .moving` before the `dec [hl]` so a delay of 0 means "move immediately" instead of wrapping. +4 bytes in bank $01.

**Tests**: 3 tests in `tests/tests/npc_movement.rs` — `ld a, [hl] / and a / jr z` at `.tickMoveCounter`, jr z targets `.moving`, `dec [hl] / jr nz` preserved after fix.

**Reference**: [Glitch City Wiki](https://glitchcity.wiki/wiki/NPC_walking_behavior_glitches)

### Binoculars NPC freeze

**File**: `home/text_script.asm` (`DisplayTextID`), `scripts/Route12Gate2F.asm` (`GateUpstairsScript_PrintIfFacingUp`)

In the gate 2F maps (Route 12, 15, 16, 18), interacting with binoculars from the side (not facing up) calls `GateUpstairsScript_PrintIfFacingUp`, which sets `wDoNotWaitForButtonPressAfterDisplayingText = TRUE` and returns without displaying text. `DisplayTextID` then enters `HoldTextDisplayOpen`, which loops while the A button is held — freezing all NPC sprite movement indefinitely, since `DisplayTextIDInit` disables sprite updates for the duration of the text interaction.

**Our fix**: Introduce a 3-value encoding for `wDoNotWaitForButtonPressAfterDisplayingText`: 0 = normal (wait for button press), 1 = hold text open while A held, 2 = close immediately (skip both wait loops). The binocular script now sets the flag to 2 when not facing up. The HOME dispatch uses `dec a / jr z, HoldTextDisplayOpen / inc a / jr nz, CloseTextDisplay` to route each value. +3 bytes in HOME (offset by 1 byte saved via `xor a` optimization in `CloseSRAM`). 0 bytes in banked ROM (only the immediate operand changes).

**Tests**: 10 tests in `tests/tests/binoculars.rs` — dispatch opcodes and jump targets, banked `ld a, 2` operand, facing-up path `xor a`, `CloseSRAM` `xor a` optimization.

**Reference**: [Glitch City Wiki](https://glitchcity.wiki/wiki/Binoculars_NPC_Pokemon_Yellow)

### Trainers' end battle text 2 isn't read correctly

**File**: `home/trainers.asm` (`ReadTrainerHeaderInfo`, `TalkToTrainer`)

Each trainer header has two end-battle text pointers (offsets `$8` and `$a`), intended for win and lose text respectively. `ReadTrainerHeaderInfo` has a special case for offset `$a` that reads the pointer into DE instead of HL. However, the function's `.done` epilogue always does `pop de` (restoring the caller's saved DE), immediately destroying the value just read. The caller in `TalkToTrainer.trainerNotYetFought` then uses `push de` / `pop de` around the offset `$8` read, passing garbage to `SaveEndBattleTextPointers` as the lose text pointer.

In practice the bug is mostly harmless because the `trainer` macro in `macros/scripts/maps.asm` duplicates the same pointer for both offsets (`dw \4, \4`), so both win and lose text would be identical anyway. But `GetSavedEndBattleTextPointer` does check `wBattleResult` and tries to use the lose pointer — with the bug, it reads garbage.

**Our fix**: Remove the custom DE handler for offset `$a` and let it fall through to `.readPointer` (reads into HL, same as all other offsets). In the caller, replace `push de` / `pop de` with `ld d, h` / `ld e, l` to copy the lose-text pointer from HL to DE before reading the win text. Saves 5 bytes in ROM0.

**Tests**: 10 tests in `tests/tests/trainer_text.rs` — `cp $a` / `jr nz, .done` routing, no DE handler remnant, `.readPointer` reads into HL, `.done` pops DE, `ld d, h` / `ld e, l` in caller, win text read targets, `SaveEndBattleTextPointers` call.

**Reference**: [pret/pokered wiki](https://github.com/pret/pokered/wiki/%5BARCHIVED%5D-Bugs-and-Glitches)

### Poké Doll bypasses ghost Marowak battle

**File**: `engine/items/item_effects.asm` (`ItemUsePokeDoll`)

The ghost Marowak on Pokémon Tower 6F is a scripted battle that must be won to progress. Using a Poké Doll in battle sets `wEscapedFromBattle` to 1, ending the battle loop, but leaves `wBattleResult` at 0 (initialized as "won" at battle start). The post-battle script in `PokemonTower6F.asm` checks `wBattleResult == 0` and sets `EVENT_BEAT_GHOST_MAROWAK`, allowing the player to skip acquiring the Silph Scope entirely — a major sequence break.

The Poké Ball code already has a ghost battle check (`callfar IsGhostBattle` / `jp z, .setAnimData`) that prevents catching the ghost, but `ItemUsePokeDoll` had no such check.

**Our fix**: Add `callfar IsGhostBattle` / `jp z, ItemUseNotTime` before the escape logic. When in a ghost battle (Pokémon Tower + wild battle + no Silph Scope), the Poké Doll is rejected with "OAK: <PLAYER>! This isn't the time to use that!" — same as other unusable-item situations. +11 bytes in bank $03.

**Tests**: 6 tests in `tests/tests/pokedoll.rs` — bank check, battle check sequence, `callfar IsGhostBattle` encoding, `jp z, ItemUseNotTime` target, escape logic preserved, total size (26 bytes).

**References**: [Glitch City Wiki — Go past the Marowak ghost without a Silph Scope](https://glitchcity.wiki/wiki/Go_past_the_Marowak_ghost_without_a_Silph_Scope) · [Bulbapedia — Marowak (ghost)](https://bulbapedia.bulbagarden.net/wiki/Marowak_(ghost))

### Repel effect override (weaker repel wastes stronger repel's steps)

**File**: `engine/items/item_effects.asm` (`ItemUseRepelCommon`)

Using a Repel, Super Repel, or Max Repel while one is already active unconditionally overwrites `wRepelRemainingSteps`, wasting any remaining steps from the previous use. From Generation II onwards, using a repel while one is active is blocked with a message.

**Our fix**: After the battle check, read `wRepelRemainingSteps` and check if nonzero. If active, jump to `ItemUseFailed` with "A repel's effect is still active!" message. +12 bytes code + 5 bytes text entry in bank $03.

**Tests**: 7 tests in `tests/tests/repel_override.rs`

**References**: [Bulbapedia — Repel effect override](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Repel_effect_override)

### Repel saving oversight (repel effect lost on save/reload)

**File**: `engine/menus/save.asm` (`SaveMainData`, `LoadMainData`)

`wRepelRemainingSteps` ($D0DA) is outside all saved WRAM blocks. Repel effect is lost on save/reload. From Generation II onwards, repel steps are saved.

**Our fix**: Add `sRepelRemainingSteps` to SRAM (within checksummed range). Save/load alongside `hTileAnimations`. +6 bytes save + 6 bytes load + 1 byte SRAM in bank $1C.

**Tests**: 5 tests in `tests/tests/repel_save.rs`

**References**: [Bulbapedia — Repel saving oversight](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Repel_saving_oversight)

### Repel step counting oversight (direction change wastes repel steps)

**File**: `engine/battle/wild_encounters.asm` (`TryDoWildEncounter`)

`TryDoWildEncounter` decrements `wRepelRemainingSteps` on both direction changes and actual movement. Turning in place wastes a repel step. From Generation II onwards, only actual steps count.

**Our fix**: Check `BIT_TURNING` in `wMiscFlags` before decrementing. If turning, skip decrement (repel still filters encounters). +10 bytes in bank $04.

**Tests**: 5 tests in `tests/tests/repel_step_count.rs`

**References**: [Bulbapedia — Repel step counting oversight](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Repel_step_counting_oversight)

### Catch rate RNG oversight

**File**: `engine/items/item_effects.asm` (`ItemUseBall`)

The capture algorithm uses rejection sampling to constrain the first random number (Rand1) to a ball-type-dependent range: [0,255] for Poké Balls, [0,200] for Great Balls, [0,150] for Ultra/Safari Balls. The loop calls `Random` repeatedly until the value fits within range. Because the Gen I RNG (`Random_`) is rDIV-based — each call reads the hardware divider register, which increments at a fixed rate — consecutive calls produce deterministically correlated values. The number of rejection loop iterations determines the exact rDIV state when Rand1 is accepted, which in turn determines rDIV's state when Rand2 is generated (fixed cycle count between them). This creates a strong correlation between Rand1 and Rand2, causing significant catch rate bias.

Effects include: Ultra Balls performing worse than Poké Balls against high-catch-rate Pokémon at full HP, Safari Zone Pokémon (Chansey, Tauros, Kangaskhan, Scyther, Pinsir, etc.) being much harder to catch than intended, and Mewtwo being literally impossible to catch after a rejection loop iteration with an Ultra Ball.

**Our fix**: Replace the rejection sampling loop with multiplication-based range reduction. Instead of looping until `Random <= B`, compute `Rand1 = Random * (B+1) / 256` using the existing `Multiply` routine. This maps [0,255] uniformly onto [0,B] with exactly one RNG call, eliminating the timing-dependent correlation between Rand1 and Rand2. Scale factors: 151 for Ultra/Safari Ball (→ [0,150]), 201 for Great Ball (→ [0,200]). Poké Ball path is unchanged (no scaling needed). +12 bytes in bank $03.

**Tests**: 8 tests in `tests/tests/catch_rate.rs` — bank check, single Random call at .loop, no rejection jr back to .loop, scale factor 201 for Great Ball, scale factor 151 for Ultra/Safari, call Multiply in scaling path, hProduct+2 read for high byte, Poké Ball bypasses scaling.

**References**: [Bulbapedia — Catch rate RNG oversight](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Catch_rate_RNG_oversight) · [Glitch City Wiki — RNG correlation (Generation I)](https://glitchcity.wiki/wiki/RNG_correlation_(Generation_I))

### Glitch Pokémon corrupt SRAM (MissingNo.)

**File**: `home/uncompress.asm` (`_UncompressSpriteData`)

The sprite decompression routine reads the first byte of compressed sprite data as dimensions (high nybble = width, low nybble = height in tiles). The original `and $f` mask allows up to 15 tiles per dimension, but the sprite buffers (`sSpriteBuffer1`/`sSpriteBuffer2`) in SRAM bank 0 only hold 7×7 tiles (392 bytes each). Glitch Pokémon like MissingNo. have garbage sprite data with dimensions exceeding this limit, causing decompression to overflow into adjacent SRAM — including `sHallOfFame` (4800 bytes of Hall of Fame records).

In Yellow specifically, MissingNo.'s dimension byte is `$00` (0×0 tiles), which causes the decompression loop to never terminate properly, writing arbitrary data across SRAM until the buffer pointer wraps.

**Our fix**: Change `and $f` to `and $7` for both height and width, capping dimensions at 7 tiles maximum. Add `jr nz, .heightNotZero` / `inc a` and `jr nz, .widthNotZero` / `inc a` zero-guards to prevent 0-dimension infinite loops (maps 0→1). Also optimize the adjacent `wSpriteCurPosX`/`wSpriteCurPosY` clearing with `ld [hli], a` / `ld [hl], a` instead of two separate `ld [nn], a`, saving 1 byte in HOME. Net cost: +5 bytes in HOME.

**Tests**: 8 tests in `tests/tests/missingno.rs` — `and $7` masks for height and width, `jr nz` / `inc a` zero-guards for both dimensions, `add a` ×3 multiply-by-8 sequences, `ld [hli], a` + `ld [hl], a` optimization, bank 0 (HOME) check.

**References**: [Bulbapedia — MissingNo.](https://bulbapedia.bulbagarden.net/wiki/MissingNo.) · [Glitch City Wiki — MissingNo.](https://glitchcity.wiki/wiki/MissingNo.)

### Item duplication glitch (MissingNo. Pokédex seen flag overflow)

**Files**: `engine/battle/core.asm` (`LoadEnemyMonData`), `engine/pokemon/add_mon.asm` (`_AddPartyMon`, `_AddEnemyMonToPlayerParty`)

When encountering MissingNo. or other glitch Pokémon, `IndexToPokedex` returns Pokédex number 0 (invalid). The code then does `dec a` (wrapping 0→255) and calls `FlagAction` with bit index 255 on the `wPokedexSeen` bitfield (19 bytes for 151 Pokémon). Bit 255 maps to byte 31 (255/8) of the array — 12 bytes past its end, landing exactly on the 6th bag item's quantity byte in `wBagItems`. Setting bit 7 adds 128 to that quantity. The same overflow affects `wPokedexOwned` when catching/boxing a glitch Pokémon, corrupting different adjacent WRAM.

**Our fix**: After `IndexToPokedex` returns, check `and a` on the Pokédex number. If 0 (invalid), skip the `FlagAction` call entirely. Applied at all three call sites: `LoadEnemyMonData` (+3 bytes bank $0F), `_AddPartyMon` (+3 bytes bank $03), and `_AddEnemyMonToPlayerParty` (+3 bytes bank $03). Total: +9 bytes across banked ROM.

**Tests**: 8 tests in `tests/tests/item_duplication.rs` — bank checks, `and a / jr z` guard present at all 3 sites, `jr z` targets correct skip labels, `dec a` follows on normal path, skip labels exist in banked ROM.

**References**: [Bulbapedia — Item duplication glitch](https://bulbapedia.bulbagarden.net/wiki/Item_duplication_glitch) · [Glitch City Wiki — Old man glitch](https://glitchcity.wiki/wiki/Old_man_glitch)

### Glitch moves have variable PP and garbage data

**Files**: `engine/events/heal_party.asm`, `engine/items/item_effects.asm`, `engine/pokemon/add_mon.asm`, `engine/pokemon/learn_move.asm`, `engine/pokemon/evos_moves.asm`, `engine/battle/core.asm`, `engine/battle/trainer_ai.asm`

Move data is stored in the `Moves` table (165 entries × 6 bytes: animation, effect, power, type, accuracy, PP). Each call site converts the 1-based move ID to a 0-based index with `dec a`, then uses `AddNTimes` to compute the table offset. For glitch moves (ID > NUM_ATTACKS = $A5 / 165), the index exceeds the table bounds and reads into `BaseStats` data, producing garbage PP values, effect IDs, type, and power. `NO_MOVE` (ID 0) wraps to $FF after `dec a`, also reading garbage. This causes variable PP display and unpredictable move behavior.

**Our fix**: At each of the 7 call sites, add `cp NUM_ATTACKS / jr c, .validMoveId / xor a` after `dec a`. This clamps out-of-range indices to 0 (POUND), ensuring valid move data is always read. POUND (effect `EFFECT_NORMAL_HIT`, 35 PP, Normal type) is a safe fallback. +5 bytes per site, 35 bytes total across banked ROM (zero impact on HOME).

**Tests**: 8 tests in `tests/tests/glitch_moves.rs` — `cp $A5 / jr c / xor a` pattern verified at all 7 sites (`HealParty`, `GetMaxPP`, `AddPartyMon_WriteMovePP`, `DontAbandonLearning`, `WriteMonMoves`, `GetCurrentMove`, `ReadMove`), plus bank check confirming all sites are in banked ROM.

**References**: [Bulbapedia — Glitch move](https://bulbapedia.bulbagarden.net/wiki/Glitch_move) · [Glitch City Wiki — Glitch move](https://glitchcity.wiki/wiki/Glitch_move)

### Super Glitch (move name buffer overflow)

**Files**: `engine/battle/core.asm` (`GetCurrentMove`, `EnemyCanExecuteChargingMove`), `engine/battle/misc.asm` (`FormatMovesString`)

Super Glitch moves (hex $A6-$C3) have no names in the `MoveNames` table. When `GetName` looks up the name, it scans past the table into ROM counting `@` terminators. The resulting garbage bytes, when displayed by `PlaceString`/`CopyString` (which also copy until `@`), overflow the screen buffer and corrupt adjacent WRAM — causing the TMTRAINER effect, HP bar corruption, team corruption, map corruption, and freezes. The fight menu (viewing move names) is the classic Super Glitch trigger.

The existing "Glitch moves have variable PP" fix clamps move IDs for the `Moves` data table (PP/effect/power) at 7 sites, but does NOT protect the separate `MoveNames` table lookup. Three code paths call `GetName` directly (bypassing `GetMoveName`), using unclamped move IDs for name display.

**Our fix**: Clamp move IDs to STRUGGLE ($A5) before `ld [wNameListIndex], a` in all 3 direct `GetName` callers: `GetCurrentMove` (unified with existing Moves-table clamp, replaces `xor a` with `ld a, STRUGGLE`), `EnemyCanExecuteChargingMove` (new clamp), and `FormatMovesString` (new clamp — the fight menu path). The 14+ callers via `GetMoveName` are already safe because the 7 data-table clamps prevent glitch IDs from entering the party/battle move slots. +13 bytes in banked ROM (banks $0E/$0F), +0 bytes HOME.

**Tests**: 5 tests in `tests/tests/super_glitch.rs` — clamp pattern (`cp $A6 / jr c / ld a, $A5`) verified at `GetCurrentMove.validMoveId`, `EnemyCanExecuteChargingMove.validMoveId`, `FormatMovesString.validMoveId`, plus bank checks ($0E/$0F).

**Reference**: [Bulbapedia — Super Glitch](https://bulbapedia.bulbagarden.net/wiki/Super_Glitch) · [Glitch City Wiki — Super Glitch](https://glitchcity.wiki/wiki/Super_Glitch) · [Bulbapedia — List of battle glitches: Super Glitch](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Super_Glitch)

### Yami Shop glitch (item name buffer overflow)

**File**: `home/names2.asm` (`GetName`)

Glitch items with unterminated names (no `@`/$50 within 20 bytes) cause a buffer overflow when displayed. `GetName` copies `NAME_BUFFER_LENGTH` (20) bytes from the name table into `wNameBuffer` via `CopyData`. For valid items, the name includes an `@` terminator. For glitch items, no `@` exists in the copied data. When `PlaceString` later displays the name, it copies bytes from `wNameBuffer` until it finds `@` — since there is none, it reads past the buffer into adjacent WRAM ($CD80+ — cached screen data), writing this data to the destination and eventually corrupting the Poké Mart item list at $CF7B+, encounter data, and other state. This is the item-name equivalent of the Super Glitch (which affects move names).

**Our fix**: After `CopyData` in `GetName`, force-write `@` at the last byte of `wNameBuffer` (`ld a, "@"` / `dec de` / `ld [de], a`). This ensures the buffer is always `@`-terminated regardless of the source data. The fix protects all name types (items, moves, trainers, etc.) going through the `.otherEntries` path. Note: `GetMonName` already had its own explicit termination (`ld [hl], '@'`). +4 bytes HOME, offset by 4 tail-call optimizations (`call BankswitchCommon / ret` → `jp BankswitchCommon` in `names2.asm`, `audio.asm`, `item_price.asm`, `npc_movement.asm`). Net: +0 bytes HOME.

**Tests**: 6 tests in `tests/tests/yami_shop.rs` — bank 0 check, fix pattern ($3E $50 $1B $12) present in GetName, fix comes after `call CopyData`, fix immediately before `.gotPtr`, `jp BankswitchCommon` tail-call at end of GetName, cross-reference `GetMonName` also terminates buffer.

**Reference**: [Glitch City Wiki — Yami Shop glitch](https://glitchcity.wiki/wiki/Yami_Shop_glitch)

### ZZAZZ glitch (prize money BCD overflow)

**File**: `engine/battle/read_trainer_party.asm` (`ReadTrainer.LastLoop`)

`ReadTrainer.LastLoop` calculates prize money by repeatedly calling `AddBCD` to add `wTrainerBaseMoney` to `wAmountMoneyWon` (a 3-byte BCD value), once per enemy level. When the BCD addition overflows `$9999`, `AddBCD`'s overflow handler writes `$99` to cap the value, but advances the DE pointer past `wAmountMoneyWon` in the process. The original code used `inc de / inc de` to restore DE for the next iteration — which only works when there is no overflow. With overflow, DE drifts forward by 3 bytes per iteration, spraying `$99` across WRAM. Since `$99` decodes to "Z" in the game's character encoding, the player's name becomes "ZZAZZ", party Pokémon are corrupted to level 153 Bulbasaur with Explosion, and hundreds of other variables are destroyed.

**Our fix**: Replace `inc de / inc de` with `ld de, wAmountMoneyWon + 2` to unconditionally reload DE after each `AddBCD` call, preventing pointer drift regardless of overflow. +1 byte in bank $0E.

**Tests**: 5 tests in `tests/tests/zzazz.rs` — bank $0E check, `ld de, nn` opcode at correct position (working backward from `SpecialTrainerMoves`), operand matches `wAmountMoneyWon + 2`, loop control intact (`dec b / jr nz / ret`), negative test (no consecutive `inc de; inc de` in `.LastLoop`).

**References**: [Bulbapedia — ZZAZZ glitch](https://bulbapedia.bulbagarden.net/wiki/ZZAZZ_glitch) · [Glitch City Wiki — ZZAZZ glitch](https://glitchcity.wiki/wiki/ZZAZZ_glitch) · [Bulbapedia — List of battle glitches: ZZAZZ glitch](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#ZZAZZ)

### Inverted sprite (wSpriteFlipped persistence)

**File**: `home/pokemon.asm` (`LoadFrontSpriteByMonIndex`)

`LoadFlippedFrontSpriteByMonIndex` sets `wSpriteFlipped = 1`, then falls through to `LoadFrontSpriteByMonIndex` which validates the Pokédex number. If the number is invalid (0 or >151, as with glitch Pokémon), the `.invalidDexNumber` path loads RHYDON and returns immediately — without clearing `wSpriteFlipped`. The flag stays set, causing all subsequent sprites (battle, status, Pokédex, Trainer Card) to render horizontally inverted. The effect persists until something explicitly clears `wSpriteFlipped` (e.g., viewing a valid Pokémon's stats).

**Our fix**: Add `xor a / ld [wSpriteFlipped], a` before `ret` in the `.invalidDexNumber` path. Also apply 5 `call BankswitchCommon / ret` → `jp BankswitchCommon` tail-call optimizations in HOME to offset the +4 byte cost (net +4 fix, −5 tail-call = −1 byte HOME).

**Tests**: 4 ROM byte tests in `tests/tests/inverted_sprite.rs` — HOME bank check, `xor a / ld [wSpriteFlipped], a / ret` present in `.invalidDexNumber` path, existing clear in `.validDexNumber` path preserved, `ld a, RHYDON` still present.

**References**: [Bulbapedia — Inverted sprite](https://bulbapedia.bulbagarden.net/wiki/Inverted_sprite) · [Glitch City Wiki — Inverted sprites](https://glitchcity.wiki/wiki/Inverted_sprites)

### Safari Zone escape via save-reset ("Glitch City")

**File**: `scripts/SafariZoneGate.asm` (`SafariZoneGateDefaultScript`)

The player can escape the Safari Zone while the step counter is still active by saving inside the Safari Zone, resetting, and walking back to the gate. `SafariZoneGateDefaultScript` only checks south-side coordinates (3,2) and (4,2) near the entrance worker, so the player at the north exit (returning from Safari Zone at coordinates (3,0) and (4,0)) bypasses the script and walks out freely with `EVENT_IN_SAFARI_ZONE` still set. The step counter then counts down in the overworld, warping the player to "Glitch City" on expiry — a corrupted map loaded from whatever bank happens to be active.

**Our fix**: Add a coordinate check at the top of `SafariZoneGateDefaultScript` for the north exit positions (3,0) and (4,0). First, `CheckEvent EVENT_IN_SAFARI_ZONE` determines if the player is returning from the Safari Zone. If set, `ArePlayerCoordsInArray` checks against `.PlayerReturningFromSafariZoneCoordsArray`. If the player is at the north exit, the script redirects to `SafariZoneGateLeavingSafariScript` (script index 5), which shows the "leaving early?" dialog. If neither condition is met, the original worker coordinate check at `.notReturningFromSafari` runs as before. +18 bytes in bank $1D.

**Tests**: 8 tests in `tests/tests/safari_escape.rs` — bank $1D check, `CheckEvent EVENT_IN_SAFARI_ZONE` at script start (opcode bytes $FA $8F $D7 $CB $7F), `jr z` targets `.notReturningFromSafari`, `call ArePlayerCoordsInArray` for return coords, `ld a, 5` sets leaving safari script, returning coords array (3,0)/(4,0) with terminator, original worker coords (3,2)/(4,2) preserved, `.notReturningFromSafari` loads worker coords.

**Note**: This fix also prevents the **Cable Club escape glitch** (Safari Zone method). The glitch requires escaping the Safari Zone with the step counter running, then entering the Cable Club and waiting for the timer to expire — warping the player out while `wLinkState` remains set. Our coordinate check blocks the initial escape, making this exploit impossible. The other trigger method (poison blackout) was already fixed in vanilla Yellow (`ApplyOutOfBattlePoisonDamage` checks `BIT_LINK_CONNECTED`).

**References**: [Glitch City Wiki — Safari Zone exit glitch](https://glitchcity.wiki/wiki/Safari_Zone_exit_glitch) · [Glitch City Wiki — Glitch City](https://glitchcity.wiki/wiki/Glitch_City) · [Bulbapedia — Glitch City](https://bulbapedia.bulbagarden.net/wiki/Glitch_City) · [Bulbapedia — Cable Club escape glitch](https://bulbapedia.bulbagarden.net/wiki/Cable_Club_escape_glitch) · [Glitch City Wiki — Cable Club escape glitch](https://glitchcity.wiki/wiki/Cable_Club_escape_glitch)

### Walking Through Walls glitch

**File**: `engine/overworld/clear_variables.asm` (`ClearVariablesOnEnterMap`)

When the Safari Zone step counter expires during a ledge jump, `SafariZoneGameOver` warps the player to the Safari Zone Gate via `WarpFound2` → `EnterMap`. However, `ClearVariablesOnEnterMap` never cleared `BIT_LEDGE_OR_FISHING` (bit 6) in `wMovementFlags` ($D735) or zeroed `wSimulatedJoypadStatesIndex` ($CD38). `CollisionCheckOnLand` checks both of these first — if `BIT_LEDGE_OR_FISHING` is set, or if `wSimulatedJoypadStatesIndex` is non-zero, tile collision is unconditionally skipped. Because the ledge jump animation was interrupted by the forced warp, the normal cleanup code (in `_HandleMidJump` / `player_animations.asm`) that clears these flags never runs. The player arrives at the Safari Zone Gate with collision permanently disabled, allowing them to walk through walls on all outdoor maps until saving and reloading (which resets `wMovementFlags`). The museum guide method exploits the same mechanism — the Safari Zone timer expires during the Pewter Museum guide's simulated walk, leaving `wSimulatedJoypadStatesIndex` non-zero after the warp.

**Our fix**: Add `res BIT_LEDGE_OR_FISHING, [hl]` on `wMovementFlags` and `ld [wSimulatedJoypadStatesIndex], a` (a=0, preserved from the preceding `FillMemory` call) to `ClearVariablesOnEnterMap` after the existing `FillMemory` call. This ensures collision-related state is always clean on map entry, regardless of what triggered the warp. +8 bytes in bank $03.

**Note**: The Safari Zone exit glitch fix (above) already blocks the primary trigger by preventing the step counter from persisting outside the Safari Zone. This fix provides defense-in-depth — even if some other mechanism causes a forced warp mid-ledge-jump, collision state will be properly reset.

**Tests**: 6 tests in `tests/tests/walk_through_walls.rs` — bank $03 check, `ld hl, wMovementFlags` present in `ClearVariablesOnEnterMap`, `res 6, [hl]` (BIT_LEDGE_OR_FISHING) follows immediately, `ld [wSimulatedJoypadStatesIndex], a` present, fix comes after `call FillMemory`, `CollisionCheckOnLand` cross-reference confirms `bit 6, a` is the collision bypass, `wMovementFlags` is outside the FillMemory range (confirming the flag was never cleared before this fix).

**References**: [Bulbapedia — Walking through walls](https://bulbapedia.bulbagarden.net/wiki/Walking_through_walls) · [Glitch City Wiki — Walk through walls trick (ledge method)](https://glitchcity.wiki/wiki/Walk_through_walls_trick_(ledge_method)) · [Glitch City Wiki — Walk through walls trick (museum guy method)](https://glitchcity.wiki/wiki/Walk_through_walls_trick_(museum_guy_method))

### Pikachu off-screen glitch (follow command buffer overflow)

**File**: `engine/pikachu/pikachu_follow.asm` (`AppendPikachuFollowCommandToBuffer`)

When certain in-game events make Pikachu "stay" in place (Jigglypuff putting Pikachu to sleep in Pewter City's Pokémon Center, Pikachu meeting Bill at Cerulean Cape, Pikachu falling in love with Clefairy at the Pokémon Fan Club), the player can walk away while Pikachu remains at the event location. `AppendPikachuFollowCommandToBuffer` records the player's walking direction (1=South, 2=North, 3=West, 4=East) into a 16-byte ring buffer at `wPikachuFollowCommandBuffer` ($D437), indexed by `wPikachuFollowCommandBufferSize` ($D436). However, the function increments the size and writes without any bounds check. After 16 steps, writes overflow past the buffer into adjacent WRAM: `wExpressionNumber` ($D447), `wPikachuMovementFlags` ($D44C), Pikachu happiness ($D46F), NPC trainer data ($D4E3+), sign coordinate arrays, and eventually save-critical data — causing NPC corruption, forced Glitch City, and save file deletion.

**Our fix**: Replace `inc [hl]` with `ld e, [hl]` / `inc e` / `bit 4, e` / `ret nz` — checking if the incremented index has bit 4 set (>= 16) and returning early without writing. This caps the buffer at exactly 16 entries, preventing any overflow. +4 bytes in bank $3F.

**Tests**: 6 tests in `tests/tests/pikachu_offscreen.rs` — bank $3F check, `bit 4, e` ($CB $63) / `ret nz` ($C0) bounds check present, `ld hl, wPikachuFollowCommandBufferSize` at function start, `ld e, [hl]` / `inc e` replaces unbounded `inc [hl]`, buffer base address cross-reference, buffer size = 16 validation.

**Reference**: [Glitch City Wiki — Pikachu off-screen glitch](https://glitchcity.wiki/wiki/Pikachu_off-screen_glitch)

### Elevator same-floor animation (lift goes to current floor)

**File**: `engine/events/elevator.asm` (`DisplayElevatorFloorMenu`)

In any elevator (Celadon Dept Store, Rocket Hideout, Silph Co.), selecting the floor the player is already on plays the full shake animation and "warps" to the same map. `DisplayElevatorFloorMenu` sets `BIT_CUR_MAP_USED_ELEVATOR` and updates `wWarpEntries` unconditionally after a floor is selected, without checking if the destination matches the floor the player entered from.

**Our fix**: After loading the destination map ID from `wElevatorWarpMaps[index]` into register C, compare with `wWarpedFromWhichMap`. If they match (`cp c` / `ret z`), skip the elevator flag and warp entirely. The destination load was moved before the flag set to enable this check. +5 bytes in bank $07.

**Tests**: 5 tests in `tests/tests/elevator_same_floor.rs` — bank $07 check, `ld a, [wWarpedFromWhichMap]` present, `cp c` / `ret z` follows, destination loaded before flag set, no old pattern (ret c immediately followed by flag set).

**Reference**: [Bulbapedia — List of overworld glitches (Generation I)](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I)

### NPC movement byte lookup carry bug

**File**: `engine/overworld/movement.asm` (`UpdateNPCSprite`)

`UpdateNPCSprite` computes the address of an NPC's movement byte in `wMapSpriteData` by converting the sprite offset to a 0-based index (`swap a / dec a / add a`), loading the base address into HL, and adding the offset to L (`add l / ld l, a`). If this addition overflows past $FF (carries), the carry is never propagated to H. The resulting address is wrong — HL points into the wrong page of WRAM — causing the NPC to read another NPC's movement byte (or garbage) and behave incorrectly.

Whether the bug triggers depends on the low byte of `wMapSpriteData` and the number of sprites on the map. In the current ROM layout ($D4E3), the maximum offset is $1C (sprite 15), giving $E3 + $1C = $FF — no carry. But the code is structurally incorrect and would break if the WRAM layout shifts (e.g. adding variables before `wMapSpriteData`).

**Our fix**: Add `jr nc, .noCarry` / `inc h` after `ld l, a` so the carry propagates to the high byte of HL. +3 bytes in bank $01.

**Tests**: 8 tests in `tests/tests/npc_movement_byte.rs` — bank $01 check, sprite offset computation sequence (`swap a / dec a / add a`), `ld hl, wMapSpriteData` base address, `add l / ld l, a` offset addition, `jr nc` carry fix ($30 $01), `inc h` carry propagation, `ld a, [hl]` movement byte read after fix, negative test (no bare `ld l, a` → `ld a, [hl]` without carry check).

**Reference**: [Glitch City Wiki — NPC walking behavior glitches](https://glitchcity.wiki/wiki/NPC_walking_behavior_glitches)

### Unobtainable Nugget in Safari Zone Gate

**Files**: `data/events/hidden_item_coords.asm`, `data/events/hidden_events.asm`

A hidden Nugget was placed at coordinates (10, 1) in the Safari Zone Gate, but this position is in the black void outside the playable area. The Itemfinder detected it from the bottom-right corner of the map, confusing players since no item could actually be collected anywhere nearby.

**Our fix**: Remove the hidden item entries from both `hidden_item_coords.asm` (the Itemfinder coordinate) and `hidden_events.asm` (the hidden event definition). Data-only fix, saves 7 bytes total (3 bytes coord + 4 bytes event).

**Tests**: 1 test in `tests/tests/safari_gate_nugget.rs` — scan `HiddenItemCoords` table to verify no entry has map ID $9C (SAFARI_ZONE_GATE).

**Reference**: [Bulbapedia — List of overworld glitches (Generation I)](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I)

### Tile pair collision loop misalignment

**File**: `home/overworld.asm` (`CheckForTilePairCollisions`)

`CheckForTilePairCollisions` iterates over tile pair arrays (`TilePairCollisionsLand` / `TilePairCollisionsWater`) in 3-byte entries: [tileset, tile1, tile2]. In `.currentTileMatchesFirstInPair`, when tile1 matches the player's standing tile but tile2 does not match the tile in front, the code jumps back to `.tilePairCollisionLoop` with HL still pointing at tile2. The loop then reads tile2 as the next entry's tileset byte via `ld a, [hli]`, misaligning ALL subsequent reads. This causes incorrect collision detection (false matches/misses) and wasted cycles scanning garbage data.

In vanilla, the bug is latent because the existing tile pair data doesn't trigger the specific scenario frequently enough to cause visible issues. But adding new collision pairs to the arrays (common in ROM hacks) makes the misalignment consistently manifest.

**Our fix**: Change `jr .tilePairCollisionLoop` to `jr .retry` in `.currentTileMatchesFirstInPair`. The `.retry` label has `inc hl` before the loop, properly advancing HL past tile2. Zero ROM growth — only the relative jump offset changes.

**Tests**: 8 tests in `tests/tests/tile_pair_collision.rs` — bank 0 (HOME) check, loop start loads tileset, entry read uses `[hli]`, end marker `$FF` check, jr targets `.retry` (not `.tilePairCollisionLoop`) in first-pair non-match path, `.retry` has `inc hl` before loop, second-pair non-match advances past tile2 correctly, negative test (first-pair non-match does NOT target `.tilePairCollisionLoop`).

**Reference**: [Bulbapedia — List of overworld glitches in Generation I](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I)

### Trainer Fly / Mew glitch (stale BIT_SEEN_BY_TRAINER on map entry)

**File**: `engine/overworld/clear_variables.asm` (`ClearVariablesOnEnterMap`)

When a trainer spots the player, `TrainerEngage` sets `BIT_SEEN_BY_TRAINER` in `wMiscFlags` ($CD60). This flag signals the overworld loop to route execution through the trainer battle flow. Normally `EndTrainerBattle` clears this flag after the battle ends. However, if the player warps away (Fly, Teleport, Dig, or Escape Rope) after a trainer spots them but before the battle starts, the flag persists because `ClearVariablesOnEnterMap` never cleared it. On the destination map, the stale flag causes the game to enter a bad state — the overworld loop misinterprets the flag as an active trainer engagement, leading to the infamous "Trainer Fly" exploit (also known as the Mew glitch). This also prevents the **0 ERROR** (walking lag) subglitch, where NPC interactions on the stale map advance `wCurMapScript` past the script pointer table bounds, causing `CallFunctionInTable` to execute garbage data as code — producing glitchy audio, invisible textboxes, movement lag, and potential crashes.

**Our fix**: Add `ld hl, wMiscFlags` / `res BIT_SEEN_BY_TRAINER, [hl]` to `ClearVariablesOnEnterMap` before `ret`, clearing the stale flag on every map transition. +5 bytes in bank $03.

**Note**: This fix makes the Mew glitch impossible. In vanilla, Mew is only obtainable via this exploit (or Nintendo events). Our fork adds Mew as a legitimate wild encounter in Cerulean Cave B1F (see `data/wild/maps/CeruleanCaveB1F.asm`).

**Tests**: 8 tests in `tests/tests/trainer_fly.rs` — bank $03 check, `ld hl, wMiscFlags` present, `res BIT_SEEN_BY_TRAINER, [hl]` present, res comes after FillMemory call, res immediately before ret, wMiscFlags outside FillMemory range (confirms flag was never cleared before), EndTrainerBattle cross-reference (also clears flag), TrainerEngage cross-reference (sets flag).

**Reference**: [Bulbapedia — Mew glitch](https://bulbapedia.bulbagarden.net/wiki/Mew_glitch) · [Bulbapedia — 0 ERROR](https://bulbapedia.bulbagarden.net/wiki/0_ERROR) · [Glitch City Wiki — Walking lag glitch](https://glitchcity.wiki/wiki/Walking_lag_glitch)

### Stuck in wall when following Oak to his lab

**File**: `scripts/PalletTown.asm` (`PalletTownPlayerFollowsOakScript`)

After the Oak intro cutscene, an auto-movement script walks the player toward Oak's Lab. `PalletTownPlayerFollowsOakScript` advances to the next script state as soon as `wNPCMovementScriptPointerTableNum` reaches zero (movement complete), without checking whether the player actually stepped on the warp tile into the lab. If the simulated movement ended with the player one tile off (due to collision alignment), they can end up stuck in or near the wall with the script in an inconsistent state.

**Our fix**: After the movement script completes, check `EVENT_FOLLOWED_OAK_INTO_LAB`. If not set, simulate one `PAD_LEFT` press via `StartSimulatingJoypadStates` to nudge the player onto the warp tile. The script only advances to `SCRIPT_PALLETTOWN_DAISY` once the event confirms the player entered the lab. ~18 bytes in the PalletTown script bank ($06).

**Tests**: 8 tests in `tests/tests/oak_lab_stuck.rs` — bank $06 check, movement script done check (`and a` / `ret nz`), `CheckEvent EVENT_FOLLOWED_OAK_INTO_LAB` macro expansion, `jr nz` targets `.followedOak`, recovery simulates `PAD_LEFT` ($20), one-step count, `jp StartSimulatingJoypadStates` target, normal path advances to `SCRIPT_PALLETTOWN_DAISY`.

### OAM updates interrupted by VBlank (sprite tearing)

**Files**: `home/update_sprites.asm` (`UpdateSprites`), `home/vblank.asm` (`VBlank`), `ram/hram.asm`

`UpdateSprites` calls the banked `_UpdateSprites` to build the OAM (Object Attribute Memory) buffer in WRAM. If VBlank fires mid-update, the `hDMARoutine` in the VBlank handler copies the half-built buffer to OAM hardware, causing sprite flickering and corruption on the overworld.

**Our fix**: Add an `hOAMUpdateLocked` flag at $FFD9 (reusing an unnamed HRAM padding byte). `UpdateSprites` sets the flag to nonzero (reusing A=$FF from the preceding `wUpdateSpritesEnabled` store) before calling `_UpdateSprites`, and clears it with `xor a` after. The VBlank handler checks this flag before `call hDMARoutine` — if nonzero, DMA is skipped for that frame. `PrepareOAMData` is not skipped since it builds data for the *next* frame. HRAM is zeroed during init (`home/init.asm`), so the flag starts as 0 (DMA enabled). +5 bytes in `home/update_sprites.asm`, +5 bytes in `home/vblank.asm`, +0 bytes HRAM (reused padding).

**Tests**: 8 tests in `tests/tests/oam_vblank.rs` — UpdateSprites in HOME bank, VBlank in HOME bank, lock set before `call _UpdateSprites`, lock uses nonzero value (A=$FF), unlock (`xor a` + `ldh`) after `call _UpdateSprites`, VBlank checks lock before DMA (`ldh` / `and a` / `jr nz` with offset 3), `.skipOAM` lands at `ld a, BANK(PrepareOAMData)`, no second DMA call in VBlank.

### Pewter Gym youngster sprite X coordinate

**File**: `scripts/PewterCity.asm` (`PewterCityYoungsterShowsPlayerGymScript`)

In Pewter City, the youngster who guides the player to the Gym has a sprite tearing/misalignment issue when leaving. The script sets `hSpriteScreenXCoord` to `$40` when the correct value is `$50`, causing the sprite's screen X position to be 16 pixels off from its actual map position.

**Our fix**: Change `ld a, $40` to `ld a, $50`. One-byte change in bank $06.

**Tests**: 8 tests in `tests/tests/pewter_youngster.rs` — bank $06 check, address in banked range, Y screen coord is $3C, Y coord stored to hSpriteScreenYCoord, X screen coord is $50 (not $40), X coord stored to hSpriteScreenXCoord, map Y coord is 22, map X coord is 16.

**Reference**: [Glitch City Wiki — Pewter Gym skip glitch](https://glitchcity.wiki/wiki/Pewter_Gym_skip_glitch) (related youngster movement bugs)

### Oak's lab music channel cut-off

**File**: `scripts/OaksLab.asm` (`OaksLabFollowedOakScript`)

After the player follows Oak into his lab, the script clears `BIT_NO_MAP_MUSIC` in `wStatusFlags7` and immediately calls `PlayDefaultMusic`. If a V-Blank interrupt fires between the flag clear and the music initialization, one of the audio channels (Ch0, Ch1, or Ch2) can be ended before it starts, causing the lab theme to play with a missing channel.

**Our fix**: Insert `call DelayFrame` between `res BIT_NO_MAP_MUSIC, [hl]` and `call PlayDefaultMusic`. This ensures a V-Blank completes before the music engine initializes, so all channels start cleanly. +3 bytes in bank $07.

**Tests**: 8 tests in `tests/tests/oaks_lab_music.rs` — bank $07 check, address in banked range, `ld hl, wStatusFlags7` found, `res 1, [hl]` (CB 8E) follows, `call DelayFrame` follows res (THE FIX), `call PlayDefaultMusic` follows DelayFrame, full sequence is 11 bytes, DelayFrame is in HOME bank.

**Reference**: [Glitch City Wiki — Professor Oak's lab music glitch](https://glitchcity.wiki/wiki/Professor_Oak%27s_lab_music_glitch)

### Professor Oak's Poké Balls glitch (bag full silently eats items)

**File**: `scripts/OaksLab.asm` (`OaksLabOak1Text.give_poke_balls`)

When Professor Oak gives the player 5 Poké Balls, the script uses `CheckAndSetEvent EVENT_GOT_POKEBALLS_FROM_OAK` which atomically checks AND sets the event flag before calling `GiveItem`. If the player's bag is full, `GiveItem` returns with carry clear (failure) but the script never checks the carry flag — it unconditionally prints the "received Poké Balls" text. Since the event flag was already set, talking to Oak again triggers the `.come_see_me_sometimes` branch, permanently losing the Poké Balls. Normally this can't happen because delivering Oak's Parcel frees a bag slot, but after beating the Route 22 rival with a full bag (no Poké Balls, < 2 owned Pokémon), it's reachable.

**Our fix**: Replace `CheckAndSetEvent` with `CheckEvent` (check-only, does not set the flag). After `call GiveItem`, add `jr nc, .no_room_for_pokeballs` to check the carry flag. Only `SetEvent EVENT_GOT_POKEBALLS_FROM_OAK` on the success path. The `.no_room_for_pokeballs` path prints a "no room for items" message without setting the event flag, so Oak will offer the Poké Balls again next time. This matches the pattern used by all gym TM scripts (PewterGym, CeladonGym, etc.). +14 bytes in bank $07 (includes `jr nz` → `jp nz` promotion for a nearby branch pushed out of range).

**Tests**: 9 tests in `tests/tests/oaks_pokeballs.rs` — bank $07 check, `CheckEvent` (not `CheckAndSetEvent`) at script start (`ld a,[nn]` + `bit B,a` followed by `jr nz`, no `set` instruction), `jr nz` targets `.come_see_me_sometimes`, `lb bc, POKE_BALL, 5` + `call GiveItem` at correct offset, `jr nc` after GiveItem (THE FIX), `jr nc` targets `.no_room_for_pokeballs`, `SetEvent` only on success path (`ld hl` + `set`), no-room path loads `.NoRoomForPokeballsText`, text_far entry exists.

**References**: [Bulbapedia — Professor Oak's Poké Balls glitch](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Professor_Oak's_Poké_Balls_glitch)

### Victory music plays on Explosion/Self-Destruct double faint

**File**: `engine/battle/core.asm` (`FaintEnemyPokemon`)

When a wild Pokémon faints, `FaintEnemyPokemon` plays the victory music at `.wild_win` before checking whether the player's party is alive. If both the player's last Pokémon and the wild Pokémon faint simultaneously (e.g., via Explosion or Self-Destruct), the victory music plays even though the player lost the battle and will black out.

**Fix**: Before the wild/trainer branch, call `AnyPartyAlive` and push the result (`call AnyPartyAlive / ld a, d / and a / push af`). At `.wild_win`, pop and skip victory music if the party is dead (`pop af / jr z, .sfxplayed`). The trainer path also pops to balance the stack. +10 bytes in bank $0F.

**Tests**: 8 ROM byte tests in `tests/tests/victory_music.rs` verifying the `call AnyPartyAlive` placement, push/pop pattern, `jr z` target, `EndLowHealthAlarm` position, and trainer path stack balancing.

**Reference**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Battle_draw_theme_oversight)

### Silent Indigo Plateau (evolution kills victory music)

**File**: `engine/battle/end_of_battle.asm` (`EndOfBattle`)

When a Pokémon evolves during the Champion (RIVAL3) battle at Indigo Plateau, `EvolveMon` calls `StopAllMusic` (twice — once before evolution animation, once after), killing the gym leader victory music. `EvolutionAfterBattle.done` cannot restore music because `wIsInBattle` is still set (`ret nz` at its exit). The overworld's `LoadMapData` cannot restore map music because `BIT_NO_MAP_MUSIC` was set by `TrainerBattleVictory` for RIVAL3. Result: complete silence from after evolution until Professor Oak arrives and plays `Music_Cities1AlternateTempo`.

**Our fix**: After `EvolutionAfterBattle` returns in `EndOfBattle`, check if `BIT_NO_MAP_MUSIC` is set in `wStatusFlags7` AND `wEvolutionOccurred` is non-zero. If both conditions are true, replay the victory music by calling `StopAllMusic` → `PlayMusic(MUSIC_DEFEATED_GYM_LEADER)` → `Delay3`. +26 bytes in bank $04.

**Tests**: 5 ROM byte tests in `tests/tests/silent_indigo.rs` — bank $04 check, `bit BIT_NO_MAP_MUSIC, [hl]` and `ld a, [wEvolutionOccurred]` checks between `.evolution` and `.skipVictoryReplay`, `call StopAllMusic` present, `call PlayMusic` present, `.skipVictoryReplay` label ordering.

**References**: [Bulbapedia — Silent Indigo Plateau](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Silent_Indigo_Plateau) · [Glitch City Wiki — Silent Indigo Plateau](https://glitchcity.wiki/wiki/Silent_Indigo_Plateau)

### Pikachu cry in link battles (electronic cry instead of voice)

**File**: `engine/battle/core.asm` (`EnemySendOutFirstMon`)

In Pokémon Yellow, the player's starter Pikachu has a special digitized voice cry ("Pikachu!") instead of the standard synthesized electronic cry. When the player sends out Pikachu, `SendOutMon` checks `IsThisPartyMonStarterPikachu` and plays the voice via `PlayPikachuSoundClip`. However, when an enemy sends out a Pikachu (in trainer or link battles), `EnemySendOutFirstMon` always calls `PlayCry`, which plays the generic electronic cry with no Pikachu species check. This is inconsistent even when both players are playing Pokémon Yellow.

**Our fix**: Before calling `PlayCry` for the enemy, check `cp PIKACHU`. If the species is Pikachu, load `PikachuCry11` (the normal battle cry) and call `PlayPikachuSoundClip` via `callfar`. Non-Pikachu species still use `PlayCry` as before. +15 bytes in bank $0F.

**Tests**: 5 ROM byte tests in `tests/tests/pikachu_cry.rs` — bank $0F check, `cp PIKACHU` species check between `.next4` and `.notEnemyPikachu`, `ld e, PikachuCry11` index loaded, `callfar PlayPikachuSoundClip` present (`call Bankswitch`), `call PlayCry` preserved at `.notEnemyPikachu` for non-Pikachu species.

**Reference**: [Bulbapedia — Pikachu cry in link battles](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Pikachu_cry_in_link_battles)

### Link battle animation oversight (Minimize visual not applied when animations off)

**File**: `engine/battle/effects.asm` (`StatModifierUpEffect` / `UpdateStatDone`)

When battle animations are disabled, `PlayCurrentMoveAnimation` skips the Minimize animation entirely (just delays 30 frames). However, the MINIMIZED flag is still set unconditionally by the effect code afterward. This creates a state mismatch: the Pokémon is logically minimized (the flag is set, affecting gameplay) but visually appears full-size (the sprite was never changed). The full-size sprite persists until something triggers a redraw (e.g., opening menus, which then suddenly shrinks the sprite). In link battles, this could cause one player to see a full-size sprite while the other sees the minimized diamond. Gen II fixed this by always applying the visual effect regardless of the animation setting.

**Our fix**: After setting the MINIMIZED flag, check `wOptions` `BIT_BATTLE_ANIMATION`. If animations were disabled (bit 7 set), call `AnimationMinimizeMon` via `Bankswitch` to apply the visual sprite replacement. This follows the existing pattern used by Substitute (`substitute.asm:51-59`) and Transform (`transform.asm:37-45`), which both have fallback paths for animations-off. +15 bytes in bank $0F.

**Tests**: 5 ROM byte tests in `tests/tests/minimize_anim.rs` — bank $0F check, `ld a, [wOptions]` / `bit BIT_BATTLE_ANIMATION, a` / `jr z` check present, `call Bankswitch` for AnimationMinimizeMon, `ld hl, AnimationMinimizeMon` present, label ordering.

**Reference**: [Bulbapedia — Link battle animation oversight](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Link_battle_animation_oversight)

### Mimic PP glitch (max PP display shows copied move's PP)

**File**: `engine/battle/core.asm` (`PrintMenuItem`)

When Mimic copies a move, the fight menu shows the copied move's max PP instead of Mimic's. `PrintMenuItem` calls `GetMaxPP` with `BATTLE_MON_DATA`, which reads the move ID from `wBattleMonMoves`. After Mimic, this slot contains the copied move (e.g., Thunderbolt with 15 base PP). But the current PP byte (`wBattleMonPP`) was never changed — it still tracks Mimic's remaining uses (base 10). This creates displays like `9/5` (9 Mimic PP left vs Horn Drill's max of 5) or `8/35` (8 Mimic PP vs Thunderbolt's max of 35). The summary screen is unaffected because it reads from party data where Mimic is still in the slot.

**Our fix**: In `PrintMenuItem`, before calling `GetMaxPP`, compute the party move for the current slot via `AddNTimes` on `wPartyMon1Moves`. If the party move is `MIMIC`, use `PLAYER_PARTY_DATA` instead of `BATTLE_MON_DATA` for `GetMaxPP`, so it looks up Mimic's base PP (10) and applies the correct PP Up bonuses. +24 bytes in bank $0F.

**Tests**: 5 ROM byte tests in `tests/tests/mimic_pp.rs` — bank $0F check, `cp MIMIC` check present, conditional `BATTLE_MON_DATA`/`PLAYER_PARTY_DATA` pattern, `ld hl, wPartyMon1Moves` party lookup, `call AddNTimes` offset calculation.

**Reference**: [Bulbapedia — Mimic PP glitch](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Mimic_PP_glitch)

### Poison/Burn animation with 0 HP

**File**: `engine/battle/core.asm` (`HandlePoisonBurnLeechSeed`)

`HandlePoisonBurnLeechSeed` checks the status byte for BRN/PSN flags and unconditionally prints the "hurt by poison/burn" text and plays the `BURN_PSN_ANIM` animation — without first checking whether the mon's HP is already 0. A mon can reach 0 HP from confusion self-damage or recoil during the same turn, and the attacker's faint is deferred until after `HandlePoisonBurnLeechSeed` returns. The result: a 0 HP mon plays the poison/burn damage flash animation before fainting. Fixed in Pokémon Gold/Silver.

**Our fix**: At the start of `.playersTurn`, check HP with `ld a, [hli] / or [hl] / dec hl`. If HP is 0, jump to `.notLeechSeeded` (the existing faint path) to skip all residual damage processing, animation, and text. +5 bytes in bank $0F.

**Tests**: 4 ROM byte tests in `tests/tests/poison_burn_0hp.rs` — bank $0F check, `ld a, [hli] / or [hl] / dec hl / jr z` sequence at `.playersTurn`, `jr z` target is `.notLeechSeeded`, status check (`ld a, [de] / and n`) follows after HP check.

**Reference**: [Bulbapedia — Poison/Burn animation with 0 HP](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Poison/Burn_animation_with_0_HP)

### Hidden item jingle cut off by audio fade-out

**File**: `engine/events/hidden_items.asm` (`FoundHiddenItemText`)

When the player finds a hidden item, `FoundHiddenItemText` plays the `SFX_GET_ITEM_2` jingle via `PlaySoundWaitForCurrent`. If `wAudioFadeOutControl` is non-zero at that moment (e.g., map music is still fading out), the fade-out counter interferes with the jingle playback, causing it to be cut short or silenced entirely.

**Our fix**: Save and clear `wAudioFadeOutControl` before playing the jingle, then restore it afterward. `ld a, [wAudioFadeOutControl] / push af / xor a / ld [wAudioFadeOutControl], a` before the sound call, and `pop af / ld [wAudioFadeOutControl], a` after `WaitForSoundToFinish`. +12 bytes in bank $1D.

**Tests**: 8 tests in `tests/tests/hidden_item_jingle.rs` — bank $1D check, address in banked range, `ld a, [wAudioFadeOutControl]` before SFX, `push af` saves state, `xor a` + store clears counter, `SFX_GET_ITEM_2` loaded, `pop af` after WaitForSoundToFinish, restore write after pop.

### Museum fossils play cry / binoculars Articuno cry distortion

**Files**: `engine/events/hidden_events/museum_fossils2.asm` (`DisplayMonFrontSpriteInBox`), `engine/events/hidden_events/route_15_binoculars.asm` (`Route15GateLeftBinoculars`)

`DisplayMonFrontSpriteInBox` is a shared function used by the Pewter Museum fossils, Route 15 binoculars (Articuno), and Fan Club pictures (Rapidash, Fearow). It displays a Pokémon's front sprite in a pop-up window but never plays the Pokémon's cry. Meanwhile, the binoculars script calls `PlayCry` *before* `DisplayMonFrontSpriteInBox`, so the cry plays while heavy VRAM operations (sprite decompression, tile loading, animation) are still running — this can distort Articuno's cry. The museum fossils and Fan Club pictures have no cry at all.

**Our fix**: Add `call PlayCry` inside `DisplayMonFrontSpriteInBox` after `AnimateSendingOutMon` (after all VRAM work is done), with `cp FOSSIL_KABUTOPS` / `cp FOSSIL_AERODACTYL` / `jr z, .skipCry` checks to skip the cry for museum fossils. Remove the standalone `call PlayCry` from `Route15GateLeftBinoculars`. +14 bytes in bank $17, −3 bytes in binoculars = +11 bytes net.

**Tests**: 8 tests in `tests/tests/fossil_cry.rs` — bank $17 check, banked range, `cp FOSSIL_KABUTOPS` found, `cp FOSSIL_AERODACTYL` found, `jr z` follows each fossil compare, `call PlayCry` before `.skipCry`, binoculars no standalone PlayCry, PlayCry after AnimateSendingOutMon.

**Reference**: [Glitch City Wiki — Articuno binoculars cry glitch](https://glitchcity.wiki/wiki/Articuno_binoculars_cry_glitch), [Bulbapedia — Articuno binoculars cry glitch](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_(Generation_I)#Articuno_binoculars_cry_glitch)

### Mr. Fuji's house door doesn't work after rescue warp

**File**: `scripts/PokemonTower7F.asm` (`PokemonTower7FWarpToMrFujiHouseScript`)

After rescuing Mr. Fuji from Pokémon Tower 7F, the game warps the player to Mr. Fuji's house and places them on the exit mat. However, the warp script doesn't set `BIT_STANDING_ON_WARP` in `wMovementFlags`, so the door tile doesn't function until the player takes one step in any direction first. This makes the post-rescue transition feel broken.

**Our fix**: Add `ld hl, wMovementFlags` / `set BIT_STANDING_ON_WARP, [hl]` before the existing `wStatusFlags3` / `BIT_WARP_FROM_CUR_SCRIPT` set. +5 bytes in bank $18.

**Tests**: 8 tests in `tests/tests/fuji_warp.rs` — bank $18 check, banked range, `ld hl, wMovementFlags` present, `set 2, [hl]` follows, `ld hl, wStatusFlags3` present, `set 3, [hl]` follows, wMovementFlags set before wStatusFlags3, exactly 2 `ld hl` + `set` patterns.

**Reference**: [Glitch City Wiki — Mr. Fuji's house door glitch](https://glitchcity.wiki/wiki/Mr._Fuji%27s_house_door_glitch)

### Pokémon Tower 2F rival encounter coords array missing terminator

**File**: `scripts/PokemonTower2F.asm` (`PokemonTower2FRivalEncounterEventCoords`)

`PokemonTower2FRivalEncounterEventCoords` is a coordinate array scanned by `ArePlayerCoordsInArray`, which reads (Y, X) byte pairs until it encounters $FF. The array ends with `db $0F` instead of `db -1` ($FF). Since $0F ≠ $FF, the scan reads past the 5-byte array into subsequent code bytes (`PokemonTower2FDefeatedRivalScript`), potentially matching garbage coordinates. The $0F value is coincidentally the X coordinate from the first `dbmapcoord 15, 5`, suggesting a copy-paste error.

**Our fix**: Change `db $0F` to `db -1`. One-byte change, zero ROM growth.

**Tests**: 8 tests in `tests/tests/tower2f_coords.rs` — bank $18 check, banked range, first coord pair (5, 15), second coord pair (6, 14), terminator is $FF, array is exactly 5 bytes, `call ArePlayerCoordsInArray` in caller, no old $0F terminator.

**Reference**: Code audit finding (not documented on Glitch City Wiki or Bulbapedia).

### Mt. Moon B2F battle-disable softlock

**File**: `scripts/MtMoonB2F.asm` (`MtMoonB2FResetScripts`)

After beating the super nerd in Mt. Moon B2F, `BIT_NO_BATTLES` in `wStatusFlags4` is set while the player stands in the fossil area (a 4×4 grid of coords). This per-frame check in `MtMoonB2F_Script` correctly clears the flag when the player steps outside the area. However, if the player uses Escape Rope, Dig, or Teleport from within the fossil area, they leave the map with `BIT_NO_BATTLES` still set, suppressing all random encounters on every subsequent map until they return to Mt. Moon B2F and step outside the fossil zone.

**Our fix**: Add `ld hl, wStatusFlags4` / `res BIT_NO_BATTLES, [hl]` to `MtMoonB2FResetScripts`, which runs when the map's scripts are reset (e.g. after a battle loss). This ensures the flag is always cleaned up. The per-frame check in `MtMoonB2F_Script` already handles the normal case when the player is on the map. +4 bytes in bank $12.

**Tests**: 8 tests in `tests/tests/mtmoon_battles.rs` — bank $12 check, banked range, `ld hl, wStatusFlags4` in reset function, `res 4, [hl]` follows, clearing after `xor a`, clearing immediately before `MtMoonB2FSetScript`, fossil area `set 4, [hl]` still present, fossil area `res 4, [hl]` still present.

**Reference**: Code audit finding (not documented on Glitch City Wiki or Bulbapedia; documented on the archived pret/pokered wiki as "Battles can get stuck in a disabled state causing softlocks").

### Save corruption from mid-save shutoff (Pokémon duplication)

**File**: `engine/menus/save.asm` (`SaveMainData`)

`SaveMainData` writes player name, main data, sprite data, and box data to SRAM, then computes a checksum over the full `sGameData` region — but does NOT write party data. Party data is written later by `SavePartyAndDexData` as a separate operation. If power is lost between the two calls, SRAM contains the new box state (e.g. a deposited Pokémon removed) but the old party state (Pokémon still in party), with a valid checksum. On reload, the game loads this inconsistent state — the same Pokémon exists in both the party and the box, enabling duplication and other exploits (accessing Pokémon beyond the 6th slot, arbitrary code execution).

**Our fix**: Add `wPartyDataStart` → `sPartyData` copy to `SaveMainData`, after the sprite data copy and before the checksum computation. This ensures party data is always written to SRAM before the checksum is computed, so either the entire save succeeds (consistent state) or the checksum doesn't match (detected as corrupt on reload). +12 bytes in bank $1C.

**Tests**: 8 tests in `tests/tests/save_corruption.rs` — bank $1C check, banked range, `ld hl, wPartyDataStart` present, `ld de, sPartyData` follows, `ld bc` with correct party size follows, `call CopyData` follows, party copy after sprite copy, party copy before checksum.

**Reference**: [Glitch City Wiki — SRAM glitch](https://glitchcity.wiki/wiki/SRAM_glitch) | [Pokémon cloning (Generation I)](https://glitchcity.wiki/wiki/Pok%C3%A9mon_cloning_(Generation_I)) | [Bulbapedia — Cloning glitches](https://bulbapedia.bulbagarden.net/wiki/Cloning_glitches#Generations_I_and_II)

### Save dialog held on screen while A button held

**File**: `engine/menus/start_sub_menus.asm` (`StartMenu_SaveReset`)

After saving, `StartMenu_SaveReset` ends with `jp HoldTextDisplayOpen`, which keeps the "game saved" dialog on screen as long as the player holds the A button. Every other start menu action dismisses immediately via `CloseStartMenu`. This inconsistency causes a brief visual stall — the screen stays frozen on the save dialog until the player releases and re-presses A.

**Our fix**: Change `jp HoldTextDisplayOpen` → `jp CloseStartMenu` so the save dialog dismisses immediately like every other start menu action. +0 bytes (same instruction size, only the target address changes).

**Tests**: 8 tests in `tests/tests/save_reset_dialog.rs` — bank $04 check, banked range, `jp CloseStartMenu` present, `jp CloseStartMenu` at end of function, no `jp HoldTextDisplayOpen` (old bug absent), `call LoadScreenTilesFromBuffer2` present, ordering (close after load), `bit 6, a` link check preserved.

**Reference**: Code audit finding.

### Save Surf exploit (surf onto non-water tiles via save/reload)

**File**: `engine/menus/save.asm` (`LoadMainData`)

`wPlayerMovingDirection` ($D527) is inside the saved main data block ($D2F6-$DA7F) and persists through save/load. When saved while holding a D-Pad direction, the stale direction survives the reload. During `EnterMap`, `UpdatePlayerSprite` reads this stale `wPlayerMovingDirection` and propagates it to `wSpritePlayerStateData1FacingDirection` — making `GetTileAndCoordsInFrontOfPlayer` check the tile in the stale direction. If that tile is water, Surf validates it. But `wPlayerDirection` defaults to south, so `.makePlayerMoveForward` moves the player south onto a non-water tile. This mismatch enables surfing onto any tile, including the SS Anne dock (bypassing the sailor).

**Our fix**: After loading the main data block in `LoadMainData`, zero `wPlayerMovingDirection` with `xor a / ld [wPlayerMovingDirection], a`. This prevents the stale direction from propagating to the sprite facing on the next `UpdatePlayerSprite` call. +4 bytes in bank $1C.

**Tests**: 4 tests in `tests/tests/save_surf.rs` — bank $1C check, `wPlayerMovingDirection` confirmed inside main data block, `xor a / ld [wPlayerMovingDirection], a` present after `.checkSumMatched`, `wSpritePlayerStateData1FacingDirection` confirmed inside sprite data block.

**References**: [Bulbapedia — Save Surf exploit](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Save_Surf_exploit)

### Struggle bypassing PP underflow

**File**: `engine/battle/decrement_pp.asm` (`.DecrementPP`)

When a move is auto-selected by the game — after thawing from freeze, during binding/trapping move continuation (Bind, Clamp, Fire Spin, Wrap), Hyper Beam recharge, or via Metronome/Mimic — the normal Struggle check in `AnyMoveToSelect` is bypassed entirely. If the auto-selected move has 0 PP, `dec [hl]` underflows the PP byte from $00 to $3F (63 PP). Because PP and PP Up count share the same byte (bits 7-6 = PP Ups, bits 5-0 = PP), this also corrupts PP Up status: a move with 0 PP Ups gains full PP Up status, while a move with PP Ups loses one boost.

**Our fix**: Add a `PP_MASK` guard in `.DecrementPP` before `dec [hl]`: load the PP byte, mask off PP Up bits with `and PP_MASK`, and `ret z` if actual PP is already 0. This prevents the underflow at the single point where all auto-selection paths converge, matching the Gen II approach of "preventing a move from being executed if it has 0 PP." +4 bytes in bank $3D.

**Tests**: 5 tests in `tests/tests/struggle_bypass.rs` — bank $3D check (2 labels), PP_MASK guard sequence present (`ld a, [hl]` / `and PP_MASK` / `ret z`), `dec [hl]` / `ret` follow guard, Struggle check still at entry (`ld a, [de]` / `cp STRUGGLE` / `ret z`).

**Reference**: [Bulbapedia — Struggle bypassing](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Struggle_bypassing) | [Glitch City Wiki — PP underflow glitch](https://glitchcity.wiki/wiki/PP_underflow_glitch) | [Glitch City Wiki — Switch PP underflow glitch](https://glitchcity.wiki/wiki/Switch_PP_underflow_glitch) | [Glitch City Wiki — Hyper Beam automatic selection glitch](https://glitchcity.wiki/wiki/Hyper_Beam_automatic_selection_glitch) | [Glitch City Wiki — Freeze top move selection glitch](https://glitchcity.wiki/wiki/Freeze_top_move_selection_glitch)

### Substitute + Confusion/Jump Kick self-damage glitch

**File**: `engine/battle/core.asm` (`ApplyDamageToPlayerPokemon`, `ApplyDamageToEnemyPokemon`, `HandleSelfConfusionDamage`, `PrintMoveFailureText`)

When a Pokémon with a Substitute hurts itself (confusion self-hit, disobedience self-hit, or Jump Kick/Hi Jump Kick crash recoil), the damage is routed through `ApplyDamageToPlayerPokemon`/`ApplyDamageToEnemyPokemon`, which checks the user's `HAS_SUBSTITUTE_UP` flag. If the user has a Substitute, it jumps to `AttackSubstitute` — a shared function that uses `hWhoseTurn` to determine which side's Substitute to damage. During self-damage, `hWhoseTurn` has been restored to its normal "my turn" value, so `AttackSubstitute` incorrectly targets the **opponent's** Substitute HP instead of the user's own HP. If the opponent has no Substitute, no damage is dealt at all.

**Our fix**: Add `ApplyDamageToPlayerPokemonDirect` and `ApplyDamageToEnemyPokemonDirect` labels right after the Substitute check in each function. All self-damage callers (confusion, disobedience, Jump Kick crash on both sides) now jump to these Direct labels with `ld hl, wDamage + 1` setup, bypassing the Substitute check entirely. Self-damage always hits the Pokémon's own HP. +12 bytes in bank $0F (3 per caller × 4 callers, for the `ld hl` preamble; labels are 0 bytes).

**Tests**: 7 tests in `tests/tests/substitute_confusion.rs` — Direct labels in bank $0F, 17-byte offset from main entry (after Substitute check), no Substitute check at Direct entry (`ld a, [hld]`), player confusion targets Direct, Jump Kick crash player/enemy target Direct, enemy confusion targets Direct.

**Reference**: [Bulbapedia — Substitute + Confusion glitch](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Substitute_%2B_Confusion_glitch) | [Glitch City Wiki — Confusion and Substitute glitch](https://glitchcity.wiki/wiki/Confusion_and_Substitute_glitch)

### Toxic counter glitches (Leech Seed + Rest)

**Files**: `engine/battle/core.asm` (`HandlePoisonBurnLeechSeed_DecreaseOwnHP`), `engine/battle/move_effects/heal.asm` (`HealEffect_`)

Two bugs involving the Toxic N counter (`wPlayerToxicCounter`/`wEnemyToxicCounter`):

1. **Toxic + Leech Seed**: The shared subroutine `HandlePoisonBurnLeechSeed_DecreaseOwnHP` computes `maxHP/16` base damage, then unconditionally checks `BADLY_POISONED` and multiplies by the Toxic N counter. Both the poison/burn path and the Leech Seed path call this same subroutine, so Leech Seed damage incorrectly escalates each turn alongside Toxic. Leech Seed should always drain a flat `maxHP/16`.

2. **Toxic + Rest**: `HealEffect_` (Rest) clears the non-volatile status byte (`wXMonStatus = 2` for sleep) but never resets `BADLY_POISONED` in `wXBattleStatus3` or zeros `wXToxicCounter`. When the Pokémon wakes and is subsequently poisoned, burned, or Leech Seeded, the damage escalates from the stale N value.

**Our fix**:
1. Add `HandlePoisonBurnLeechSeed_DecreaseOwnHP_NoToxic` entry point using the `db $06` trick: `ld a, 1 / db $06` before the normal entry's `xor a` sets a flag in A; `push af` at entry saves it; `pop af / and a / jr nz, .noToxic` at `.nonZeroDamage` skips the Toxic multiplier when flag is set. Leech Seed caller changed to use the `_NoToxic` entry. +9 bytes bank $0F.
2. In `HealEffect_`, after `ld [hl], 2` (status clear), add `push af` to save the Z flag, then `res BADLY_POISONED, [hl]` on `wXBattleStatus3` and `xor a / ld [de], a` on `wXToxicCounter`, then `pop af` to restore the Z flag for the text branch. +21 bytes bank $3D.

**Tests**: 7 tests in `tests/tests/toxic_counter.rs` — NoToxic entry point exists (3 bytes before normal), flag set (`ld a, 1` / `db $06`), `push af` after entry, `pop af / and a / jr nz` at `.nonZeroDamage`, Leech Seed calls NoToxic, Rest has `res 0, [hl]` / `xor a` / `ld [de], a` at `.resetToxicPlayer`, banked ROM check.

**Reference**: [Bulbapedia — Toxic counter glitches](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Toxic_counter_glitches) | [Glitch City Wiki — Leech Seed and Toxic stacking](https://glitchcity.wiki/wiki/Leech_Seed_and_Toxic_stacking)

### Transform + Mirror Move/Metronome PP error

**File**: `engine/battle/core.asm` (`IncrementMovePP`)

When a transformed Pokémon uses Mirror Move or Metronome, `IncrementMovePP` increments both battle PP and party PP in the corresponding move slot. But during Transform, battle PP and party PP are independent — `DecrementPP` already skips the party decrement when `TRANSFORMED` is set. The missing check in `IncrementMovePP` causes the party slot's PP to increase by 1, even in empty move slots (which have 0 PP). This can prevent Struggle from activating or cause a softlock when targeted by Disable.

**Our fix**: Add a `TRANSFORMED` bit check (via `wPlayerBattleStatus3`/`wEnemyBattleStatus3`) before the party PP increment (`inc [hl]`), mirroring the existing guard in `DecrementPP`. If transformed, `ret nz` skips the party increment. +13 bytes in bank $0F.

**Tests**: 5 tests in `tests/tests/transform_pp.rs` — bank $0F check, `bit TRANSFORMED, [hl]` present at `.checkTransformed`, `ret nz` follows, `.checkTransformed` before `.updatePP`, `inc [hl]` / `ret` at `.updatePP` end.

**Reference**: [Bulbapedia — Transform + Mirror Move/Metronome PP error](https://bulbapedia.bulbagarden.net/wiki/List_of_Transform_glitches#Transform_%2B_Mirror_Move/Metronome_PP_error)

### Trapping sleep glitch

**File**: `engine/battle/core.asm` (`ExecutePlayerMove`, `ExecuteEnemyMove`)

When a player's Pokémon is trapped by a binding move (Wrap, Bind, Clamp, Fire Spin), `wPlayerSelectedMove` is set to `CANNOT_MOVE` ($FF) every turn. If the player uses items (instead of selecting a move via FIGHT), `wPlayerSelectedMove` stays at $FF. When the trapping ends and the enemy puts the Pokémon to sleep, `ExecutePlayerMove` sees $FF, increments to 0 (Z flag set), and skips directly to `ExecutePlayerMoveDone` — bypassing `CheckPlayerStatusConditions` entirely. Since `CheckPlayerStatusConditions` is the only place the sleep counter is decremented, the Pokémon is permanently stuck showing "fast asleep."

**Our fix**: When `wPlayerSelectedMove == CANNOT_MOVE`, still call `CheckPlayerStatusConditions` before jumping to `ExecutePlayerMoveDone`. The sleep/freeze counters decrement normally, and the Pokémon wakes up after the expected number of turns. Same fix applied to the enemy side (`ExecuteEnemyMove` / `CheckEnemyStatusConditions`). +5 bytes per side, +10 bytes total in bank $0F.

**Tests**: 4 tests in `tests/tests/trapping_sleep.rs` — player side call sequence (`inc a` / `jr nz` / `call CheckPlayerStatusConditions` / `jp ExecutePlayerMoveDone`), enemy side call sequence, bank $0F check, `.canMove` labels exist.

**Reference**: [Bulbapedia — Trapping sleep glitch](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Trapping_sleep_glitch) | [Glitch City Wiki — Trapping move and sleep glitch](https://glitchcity.wiki/wiki/Trapping_move_and_sleep_glitch)

## Already fixed in Yellow (confirmed with tests)

### Struggle PP Ups fix

**File**: `engine/battle/core.asm` (`AnyMoveToSelect`)

In Red/Blue, `AnyMoveToSelect` checked raw PP bytes without masking PP Up bits. If any move had PP Ups (upper 2 bits) but 0 actual PP, the check saw non-zero and incorrectly concluded PP was available, preventing Struggle from activating. Yellow fixed this with `and PP_MASK` in both the normal path and the disabled-move path.

**Fix**: Already applied by Game Freak in Yellow. Line 2901: `and PP_MASK` (no disabled move path). Line 2923: `and $3f` (disabled move path, with upstream comment "bugfix: only check PP value and not PP up bits").

**Tests**: 7 tests in `tests/tests/struggle.rs` confirming the fix works: PP Ups only → Struggle, multiple PP Ups → Struggle, real PP → HasPP, PP Ups + real PP → HasPP, disabled move with PP Ups only → Struggle, disabled move with real PP → HasPP.


### Rare Candy level >= 100 cap (leveling past 100 glitch)

**File**: `engine/items/item_effects.asm` (`ItemUseMedicine.useRareCandy`)

The Rare Candy level check used `cp MAX_LEVEL / jr z` — checking for exact equality with 100. A glitch-obtained Pokémon above level 100 bypasses this check, allowing Rare Candies to increment levels from 101 to 255. At level 255, `inc a` wraps the byte to 0, creating a level 0 Pokémon.

**Our fix**: Changed `jr z` ($28) to `jr nc` ($30). The `cp MAX_LEVEL` instruction sets carry if A < 100 and clears carry if A >= 100. With `jr nc`, any level >= 100 jumps to `.vitaminNoEffect`. Zero bytes added — same instruction size.

**Tests**: 5 tests in `tests/tests/rare_candy_level_cap.rs`.

**References**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_I) (Leveling past 100), [Glitch City Wiki](https://glitchcity.wiki/wiki/Experience_underflow_glitch)

### Pokémon merge glitch (species list $FF confusion)

**File**: `engine/pokemon/remove_mon.asm` (`_RemovePokemon`)

The species-shift loop in `_RemovePokemon` used `inc a / jr nz` to detect the $FF list terminator. A glitch Pokémon with species index $FF is indistinguishable from the terminator, causing the loop to exit early. The OT/nickname/struct data (shifted by address-range `CopyDataUntil`) gets out of sync with the truncated species list, creating "merged" hybrid Pokémon.

**Our fix**: Replaced the $FF-based terminator loop with a count-based loop. Before shifting, compute iteration count from the (already-decremented) count and `wWhichPokemon`: `bytes = newCount + 1 - wWhichPokemon`. Uses `push af / pop af` to preserve the count, then `dec b / jr nz` instead of `inc a / jr nz`. +5 bytes in bank $01.

**Tests**: 6 tests in `tests/tests/pokemon_merge.rs`.

**References**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_merge_glitch), [Glitch City Wiki](https://glitchcity.wiki/wiki/Pok%C3%A9mon_merge_glitch)

### Healthy party deposit (deposit last healthy Pokémon)

**File**: `engine/pokemon/bills_pc.asm` (`BillsPCDeposit`)

`BillsPCDeposit` only checks `wPartyCount > 1` before allowing deposit. It never verifies that remaining party members have HP > 0. Players can deposit all healthy Pokémon, leaving only fainted ones, causing an immediate blackout after 1 step (Yellow) or 4 steps (Red/Blue).

**Our fix**: Added `CheckDepositAllowedByHP` subroutine that iterates party HP values, skipping the selected Pokémon (`wWhichPokemon`). If all other party members have HP = 0, the deposit is blocked and `CantDepositLastMonText` is shown. Called after the user selects "Deposit" from the PC submenu. +47 bytes in bank $08.

**Tests**: 8 tests in `tests/tests/healthy_deposit.rs`.

**References**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_(Generation_I)) (Pokémon Storage System healthy party deposit)

### Pokédex assumption glitch (Oak shows Dex rating without Pokédex)

**File**: `scripts/OaksLab.asm` (`OaksLabOak1Text`)

`OaksLabOak1Text` checks `wPokedexOwned >= 2` to decide whether to show the Pokédex rating, but does NOT check `EVENT_GOT_POKEDEX`. If the player catches 2+ species before receiving the Pokédex (starter + one wild catch), Oak shows the Dex rating instead of accepting Oak's Parcel, permanently blocking game progression.

**Our fix**: Added `CheckEvent EVENT_GOT_POKEDEX / jr z, .check_for_poke_balls` after the `cp 2 / jr c` check. This matches the international Red/Blue fix exactly. +7 bytes in bank $07.

**Tests**: 6 tests in `tests/tests/pokedex_assumption.rs`.

**References**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I) (Pokédex assumption glitch), [Glitch City Wiki](https://glitchcity.wiki/wiki/Oak%27s_Parcel_prevented_progress_glitch)

### Pikachu friendship item effect (no-effect items boost happiness)

**File**: `engine/items/item_effects.asm` (`ItemUseMedicine`)

`ItemUseMedicine` called `farcall_ModifyPikachuHappiness PIKAHAPPY_USEDITEM` before checking whether the item actually had any effect. Items that fail (Potion on full HP, Antidote when not poisoned, Calcium when stat EXP is maxed) still increase Pikachu's friendship without being consumed, allowing infinite happiness grinding.

**Our fix**: Removed the premature happiness call and moved it to the two success paths: `.doneHealing` (healing items that worked) and `.gotStatName` (vitamins that boosted stats). +2 bytes in bank $03.

**Tests**: 6 tests in `tests/tests/friendship_item.rs`.

**References**: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_I#Friendship_item_effect), [Glitch City Wiki](https://glitchcity.wiki/wiki/Walking_Pikachu_happiness_glitch)

## Not fixed (could fix in the future)

See [REMAINING_GLITCHES.md](REMAINING_GLITCHES.md) for the full audit of 156 glitches.
