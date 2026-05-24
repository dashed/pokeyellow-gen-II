# Remaining Glitches Audit

Comprehensive audit of all known Generation I glitches cross-referenced against
our fix branches. Sources: [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/)
and [Glitch City Wiki](https://glitchcity.wiki/wiki/).

Last updated: 2026-03-16 (99 fixes across 8 branches, 97 total bug fixes + 2 features)

---

## New Fixable Glitches (Not Yet Implemented)

### High Priority — Clear code bugs, straightforward fixes

#### ~~1. Switch PP underflow (trapping move auto-select)~~ — ALREADY FIXED

Already fixed on `dashed/battle-bugs` as "Struggle bypassing PP underflow"
(`engine/battle/decrement_pp.asm`).  The fix adds `ld a, [hl] / and PP_MASK /
ret z` before `dec [hl]`, preventing all PP underflow paths (trapping switch,
Hyper Beam, defrost, Metronome/Mimic).  5 tests in `struggle_bypass.rs`.

- [Glitch City Wiki](https://glitchcity.wiki/wiki/Switch_PP_underflow_glitch)
- [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Struggle_bypassing)

#### ~~2. Vaporeon learnpool glitch (Yellow-exclusive)~~ — ALREADY FIXED

Already fixed on `dashed/battle-bugs` as a side effect of the "level-up learnset
skipping" fix (commit `dc9b25b5`).  The fix changed `LearnMoveFromLevelUp` to
continue iterating through the learnset after learning a move (`jr nz` → `jr c`
+ loop continuation), so all moves at the same level are now processed.

- [Glitch City Wiki](https://glitchcity.wiki/wiki/Vaporeon_learnpool_glitch)
- [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Level-up_learnset_skipping)

#### ~~3. Pokédex assumption glitch~~ — ALREADY FIXED

Fixed on `dashed/overworld-fixes` (commit `0a7738c3`).  Added
`CheckEvent EVENT_GOT_POKEDEX / jr z, .check_for_poke_balls` guard in
`OaksLabOak1Text`, matching the international Red/Blue fix.  6 tests in
`pokedex_assumption.rs`.

- [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I)
- [Glitch City Wiki](https://glitchcity.wiki/wiki/Oak%27s_Parcel_prevented_progress_glitch)

---

### Medium Priority — Need verification against existing fixes

#### ~~4. Haze + Hyper Beam permanent lock~~ — ALREADY FIXED

Already fixed on `dashed/battle-bugs` (commit `83c`).  The root cause is a
missing `call ClearHyperBeam` in the `.freeze2` path (enemy freezes player) of
`FreezeBurnParalyzeEffect`.  The fix adds the call, matching `.freeze1` (player
freezes enemy).  This prevents `NEEDS_TO_RECHARGE` from persisting through
freeze → Haze thaw, which would permanently lock the Pokémon.

- [Glitch City Wiki](https://glitchcity.wiki/wiki/Haze_glitch)
- [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Hyper_Beam_+_Freeze_permanent_helplessness)

#### ~~5. Slot machine first reel `jr c` / `jr z` typo~~ — ALREADY FIXED

Already fixed on `dashed/overworld-fixes` (commit `58db07c6`).  Changed `jr c`
($38) to `jr z` ($28) in `SlotMachine_StopWheel1Early.sevenAndBarMode`.  The
`cp HIGH(SLOTS7)` comparison now correctly stops the wheel when a 7 symbol is
visible.  8 tests in `slot_lucky.rs`.

- [Glitch City Wiki](https://glitchcity.wiki/wiki/Slot_machine_behaviors_(Generation_I))

#### ~~6. Hyper Beam auto-selection (trapping move miss)~~ — FIXED

Fixed on `dashed/battle-bugs` (commit `31e8611c`).  Removed premature
`ClearHyperBeam` from `TrappingEffect`; moved recharge clearing to
`.HeldInPlaceCheck` / `.checkIfTrapped` where the target is genuinely trapped.
5 tests in `hyper_beam_trap.rs`.

- [Glitch City Wiki](https://glitchcity.wiki/wiki/Hyper_Beam_automatic_selection_glitch)

#### ~~7. Transform empty move slot corruption~~ — ALREADY FIXED

Already fixed on `dashed/battle-bugs` (commit `70dcb274`).  The fix adds a
`TRANSFORMED` check to `IncrementMovePP` before the party PP increment,
mirroring the existing guard in `DecrementPP`.  This prevents the +1 PP leak
into empty party move slots.  5 tests in `transform_pp.rs`.

- [Glitch City Wiki](https://glitchcity.wiki/wiki/Transform_empty_move_glitch)

#### ~~8. Erroneous stone evolutions~~ — NOT APPLICABLE (Red/Blue only)

This bug only affects Red/Blue.  Yellow already contains Game Freak's fix: a
`wIsInBattle` guard in `EvolutionAfterBattle` (evos_moves.asm:95-99) skips all
`EVOLVE_ITEM` entries during post-battle evolution checks.  The comment at line
12 explicitly states: "there was a bug in red/blue that allows item evolutions
to occur which is patched here."

- [Glitch City Wiki](https://glitchcity.wiki/wiki/Evolve_without_an_evolutionary_stone)

---

## Already Fixed (96 fixes across 8 branches)

### `dashed/accuracy-crit` — 3 fixes

| Fix | Reference |
|-----|-----------|
| 1/256 miss glitch | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#1/256_miss_glitch) |
| 1/256 critical hit glitch | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Critical_hit_ratio_error) |
| Focus Energy / Dire Hit quarters crit rate | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Critical_hit_ratio_error) |

### `dashed/battle-bugs` — 36 fixes

| Fix | Reference |
|-----|-----------|
| Substitute 1/4 HP rounding | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I) |
| Dual-type damage misinformation | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Dual-type_damage_misinformation) |
| Drain/Dream Eater vs Substitute | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Substitute_HP_drain_bug) |
| Counter glitches | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Counter_glitches) |
| Bide damage errors | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Bide_errors) |
| Psywave desynchronization | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Psywave_desynchronization) |
| Fly/Dig invulnerability persists | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Invulnerability_glitch) |
| Healing moves fail (255/511 HP below max) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#HP_recovery_move_failure) |
| Switch-out message underflow | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I) |
| Haze stat reset errors | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I) |
| Exp. All experience decrease | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Exp._All_oversight) |
| CooltrainerF AI targeting error | [Glitch City Wiki](https://glitchcity.wiki/wiki/CooltrainerF_AI_glitch) |
| AI HUD update oversight | — |
| Index #000 post-capture invisible Ditto | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Index_.23000_post-capture) |
| Jump Kick / Hi Jump Kick crash damage | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Jump_Kick_/_Hi_Jump_Kick_crash_damage) |
| Level-up learnset skipping | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Level-up_learnset_skipping) |
| Mimic level-up move loss | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Mimic_level_up_glitch) |
| Mirror Move desynchronization | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Mirror_Move_glitch) |
| Psywave infinite loop (level 0/1/171) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Psywave_infinite_loop) |
| Red bar sound suppression | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Red_bar_glitch) |
| Stat modification errors (badge boost stacking) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Stat_modification_errors) |
| Struggle PP bypass / underflow | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Struggle_bypassing) |
| Substitute + Confusion self-hit | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Substitute_+_Confusion_glitch) |
| Toxic counter persists through Rest/Leech Seed | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Toxic_counter_glitches) |
| Trapping + Sleep move interaction | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Trapping_sleep_glitch) |
| Transform PP corruption (Mirror Move/Metronome) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_Transform_glitches) |
| Silent Indigo Plateau (evolution mutes music) | — |
| Pikachu cry in link battles | — |
| Minimize animation not shown when anims disabled | — |
| Mimic PP display (shows copied move's PP) | — |
| Poison/Burn animation at 0 HP | — |
| Substitute sprite vanishing | — |
| First rival Pikachu animation oversight | — |
| Experience PC withdrawal freeze (CalcLevelFromExperience infinite loop) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_I) |
| Hyper Beam + Freeze permanent lock | [Glitch City Wiki](https://glitchcity.wiki/wiki/Haze_glitch) |
| Hyper Beam + Sleep always hits during recharge | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I) |

### `dashed/ghost-battle` — 2 fixes

| Fix | Reference |
|-----|-----------|
| Ghost Pokédex seen flag set without Silph Scope | [Glitch City Wiki](https://glitchcity.wiki/wiki/Ghost_Pok%C3%A9dex_seen_flag_glitch) |
| Ghost sprite reload after battle | — |

### `dashed/item-fixes` — 9 fixes

| Fix | Reference |
|-----|-----------|
| PP restore PP Ups masking bug | — |
| Transform/Ditto catch assumption | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_Transform_glitches#Transform_assumption_oversight) |
| Status cure removes stat modifiers | — |
| Poké Doll ghost Marowak sequence break | [Glitch City Wiki](https://glitchcity.wiki/wiki/Go_past_the_Marowak_ghost_without_a_Silph_Scope) |
| Item Finder coordinate 0 exclusion | — |
| Repel effect override | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Repel_effect_override) |
| Vending machine price hardcode | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Vending_machine_glitch) |
| Catch rate RNG bias (rejection sampling) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Catch_rate_RNG_oversight) |
| Pikachu friendship item effect | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_I#Friendship_item_effect) / [Glitch City Wiki](https://glitchcity.wiki/wiki/Walking_Pikachu_happiness_glitch) |

### `dashed/overworld-fixes` — 51 fixes

| Fix | Reference |
|-----|-----------|
| Route 16 sign readable from Route 17 | — |
| Invisible tree collision wall | [Glitch City Wiki](https://glitchcity.wiki/wiki/Invisible_tree_glitch) |
| Ledge jump lands on NPC | [Glitch City Wiki](https://glitchcity.wiki/wiki/NPC_collision_bypassing_glitch) |
| Bicycle music persists through hole warp | [Glitch City Wiki](https://glitchcity.wiki/wiki/Bicycle_music_hole_glitch) |
| Escape sprite garbled tiles | [Glitch City Wiki](https://glitchcity.wiki/wiki/Escape_sprite_handling_glitch) |
| NPC movement not restricted (down/right) | [Glitch City Wiki](https://glitchcity.wiki/wiki/NPC_walking_behavior_glitches) |
| NPC offscreen border detection off-by-one | [Glitch City Wiki](https://glitchcity.wiki/wiki/NPC_walking_behavior_glitches) |
| NPC movement delay wraparound (0 → $FF) | [Glitch City Wiki](https://glitchcity.wiki/wiki/NPC_walking_behavior_glitches) |
| Binoculars NPC freeze | [Glitch City Wiki](https://glitchcity.wiki/wiki/Binoculars_NPC_Pokemon_Yellow) |
| Trainers' end battle text 2 pointer destroyed | — |
| Cycling Road guard bypass | [Glitch City Wiki](https://glitchcity.wiki/wiki/Go_on_Cycling_Road_without_a_Bicycle) |
| Game Corner 10-coin inaccessible tile | — |
| Game Corner 40-coin → 20-coin error | — |
| Safari Zone escape via save-reset | [Glitch City Wiki](https://glitchcity.wiki/wiki/Glitch_City) |
| Walking Through Walls (ledge flag + Safari timer) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Walking_through_walls) |
| Trainer Fly / Mew glitch | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Mew_glitch) |
| Oak's lab music after Pikachu event | — |
| Professor Oak's Poké Balls (bag full blocks) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I) |
| Repel steps not saved | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Repel_saving_oversight) |
| Repel step counting (direction changes) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Repel_step_counting_oversight) |
| Save Surf exploit (stale facing direction) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I#Save_Surf_exploit) |
| Hidden item jingle plays wrong sound | — |
| Slot machine tile pair collision | — |
| Lucky slot 7-stop oversight | [Glitch City Wiki](https://glitchcity.wiki/wiki/Slot_machine_behaviors_(Generation_I)) |
| Lucky slot wheel 2 early stop false positive | [Glitch City Wiki](https://glitchcity.wiki/wiki/Slot_machine_behaviors_(Generation_I)) |
| Splash screen stars alignment | — |
| Healing machine tile offset | — |
| GetName TM/HM buffer overflow | — |
| Cycling Road flags leak into new game | — |
| ED tile pair collision | — |
| Stuck-in-wall (warp to impassable tile) | — |
| OAM VBlank interruption (sprite flickering) | — |
| Pewter Gym youngster sprite X offset | [Glitch City Wiki](https://glitchcity.wiki/wiki/Pewter_Gym_skip_glitch) |
| Hidden coins (Game Corner aisle) | — |
| Museum fossil Aerodactyl cry plays Golbat | — |
| Mr. Fuji's house door requires extra step | [Glitch City Wiki](https://glitchcity.wiki/wiki/Mr._Fuji%27s_house_door_glitch) |
| Pokémon Tower 2F rival coords terminator ($0F) | — |
| Mt. Moon B2F battle-disable softlock | — |
| Save corruption (party data not in SaveMainData) | [Glitch City Wiki](https://glitchcity.wiki/wiki/SRAM_glitch) |
| Save dialog held on screen (A-hold) | — |
| Tile pair collision table errors | — |
| NPC movement carry flag oversight | — |
| Lucky slot off-by-one (cp $7 → cp $8) | [Glitch City Wiki](https://glitchcity.wiki/wiki/Slot_machine_behaviors_(Generation_I)) |
| Pikachu off-screen buffer overflow | [Glitch City Wiki](https://glitchcity.wiki/wiki/Pikachu_off-screen_glitch) |
| Elevator same-floor animation | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I) |
| Pallet Town NPC walks onto door tile | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I) |
| Safari Gate inaccessible hidden Nugget | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I) |
| Healthy party deposit (deposit last healthy mon) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_I) |
| Battle transition missing dungeon maps | — |
| Strength boulder OAM corruption | — |

### `dashed/glitch-safety` — 9 fixes

| Fix | Reference |
|-----|-----------|
| MissingNo. SRAM corruption (sprite overflow) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/MissingNo.) / [Glitch City Wiki](https://glitchcity.wiki/wiki/MissingNo.) |
| Glitch move variable PP (7 call sites) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Glitch_move) / [Glitch City Wiki](https://glitchcity.wiki/wiki/Glitch_move) |
| Super Glitch (move name buffer overflow) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Super_Glitch) / [Glitch City Wiki](https://glitchcity.wiki/wiki/Super_Glitch) |
| ZZAZZ glitch (BCD overflow) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/ZZAZZ_glitch) / [Glitch City Wiki](https://glitchcity.wiki/wiki/ZZAZZ_glitch) |
| Item duplication (Pokédex flag overflow) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Item_duplication_glitch) |
| Inverted sprite (wSpriteFlipped persistence) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Inverted_sprite) / [Glitch City Wiki](https://glitchcity.wiki/wiki/Inverted_sprites) |
| Yami Shop (item name buffer overflow) | [Glitch City Wiki](https://glitchcity.wiki/wiki/Yami_Shop_glitch) |
| Rare Candy level >= 100 cap | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_I) / [Glitch City Wiki](https://glitchcity.wiki/wiki/Experience_underflow_glitch) |
| Pokémon merge glitch (species $FF confusion) | [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_merge_glitch) / [Glitch City Wiki](https://glitchcity.wiki/wiki/Pok%C3%A9mon_merge_glitch) |

---

## Already Addressed by Prerequisite Fixes

These glitches cannot trigger because our existing fixes block the prerequisite
chain that enables them.

| Glitch | Blocked by |
|--------|-----------|
| Item underflow (expanded item pack) | Item duplication fix blocks 255-stack prerequisite |
| Arbitrary code execution (ACE) | Save corruption, item duplication, and Trainer Fly fixes block all known entry points |
| Beating the game quickly (ACE speedrun) | Same as above |
| Save data carryover (party from old save) | SaveMainData party data fix |
| Save corruption (255 party Pokémon) | SaveMainData party data fix |
| Cloning glitch (SRAM method) | SaveMainData party data fix |
| Perpetual spinning animation | Safari Zone escape fix prevents the trigger |
| Changing NPC sprites (via WTW) | Walking Through Walls fix prevents the trigger |
| Changing player sprite (via WTW) | Walking Through Walls fix prevents the trigger |
| Pokémon Zoo Chansey facing south | Walking Through Walls fix prevents approach from south |

---

## Not Applicable to Pokemon Yellow

These glitches only affect other games or platforms.

| Glitch | Why not applicable |
|--------|-------------------|
| Time Capsule exploit | Gen II bug (Poké Transporter validation) |
| Trade Evolution learnset oversight | Gen II bug (trade-back move stripping) |
| Pokémon Bank hex:FF glitch | 3DS Poké Transporter bug (external SRAM manipulation) |
| Link cable trade cloning | Unfixable hardware limitation (physical disconnect) |
| Rhydon glitch / trap | Intentional safety mechanism (preserved by design) |
| Select glitches (item/move swap) | Japanese Red/Green only (fixed in international releases) |
| Dokokashira door glitch | Japanese Red/Green only |
| Old man glitch | English Red/Blue only (Yellow uses different intro) |
| Fight Safari Zone Pokémon trick | English Red/Blue only |
| New-game Nidorino cry | Red/Blue only (Yellow has Pikachu intro) |
| Full Box glitch | Japanese Red/Green only |
| Empty Pokémon List | Japanese Red/Green only |
| Pewter Gym skipping | Red/Blue only (Yellow has different NPC behavior) |
| Rival twins | Red/Blue only |
| Statue water tile | Red/Blue only |
| Swift effect glitch | Japanese Red/Green only |
| Evolution stone bypassing | Red/Green/Blue only (not Yellow) |
| Binding move wrong-side fainting | Japanese Red/Green only |
| Introduction Nidorino cry | Red/Blue only (Yellow has different intro) |
| Trade menu palette glitch | Already fixed in Yellow |
| Instant Text trick (Bike Shop) | English Red/Blue only |
| Purple Jigglypuff | Blue only |
| Stadium-specific bugs | Pokémon Stadium, not Gen I cartridge |
| Whirlwind text box overflow | Japanese Red/Green only |

---

## Recommended Skip (Cosmetic / Design Choices)

These are minor visual quirks, intentional design decisions, or issues where the
fix complexity far outweighs the benefit.

| Glitch | Reason to skip |
|--------|---------------|
| Town Map selection oversight | Cosmetic UX quirk, no gameplay impact |
| Red's transparent white pixels (title screen) | Minor graphical oddity, cosmetic only |
| Link battle animation asymmetry | Cosmetic, only visible in link battles |
| Pikachu entering link battles animation | Cosmetic, asymmetric animation between linked games |
| Disabling NPC animation during scroll | Cosmetic, NPC sprites freeze during screen transition |
| Invisible Prof. Oak (starter sequence) | Cosmetic, sprites temporarily vanish during scripted event |
| Delayed Pikachu follow (zone transition) | Cosmetic, one-step desync after zone transitions |
| NPC over grass (Viridian Forest) | Cosmetic, sprite priority issue |
| Freezing Pikachu (ledge jump + dance) | Cosmetic, temporary sprite lock |
| Pewter City NPC disappearance | Cosmetic, gym guide walks into dead-end |
| Walking through NPC in Oak's lab | Cosmetic, scripted sequence clipping |
| Ghost Marowak randomized DVs | Working as designed (all wild encounters randomize DVs) |
| Rematching Trainers (whiteout exploit) | Design choice, complex to fix safely |
| Cut glitch — standing on a tree | Requires SRAM schema change, too complex for benefit |
| Ghost identity unveiling (party screen) | Likely covered by ghost-battle fixes, needs verification |
| Cable Club escape | Sequence break exploit, low priority |
| Ghost Bicycle glitch | Likely covered by Cycling Road flags fix |
| 0 ERROR glitch | Variant of Trainer Fly, likely covered by existing fix |

---

## Summary

| Category | Count |
|----------|-------|
| Already fixed | 103 |
| Already prevented (by prerequisite fixes) | 10 |
| New fixable (high priority) | 0 |
| New fixable (medium priority, needs verification) | 0 |
| Not applicable (wrong game/platform) | 25 |
| Recommended skip (cosmetic/design) | 18 |
| **Total glitches audited** | **156** |
