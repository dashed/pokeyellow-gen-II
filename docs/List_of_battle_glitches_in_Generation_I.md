---
title: "List of battle glitches in Generation I"
author: "Bulbapedia"
published: 2026-01-26T22:07:11Z
source: "https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I"
domain: "bulbapedia.bulbagarden.net"
language: "en"
word_count: 7133
---

*For other glitches in this generation, see [List of glitches in Generation I](https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_I "List of glitches in Generation I")*

This is a **list of [Pokémon battle](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_battle "Pokémon battle") glitches in [Generation I](https://bulbapedia.bulbagarden.net/wiki/Generation_I "Generation I") games**.

### Fixed in this ROM hack

The following battle glitches from this list are fixed in our ROM hack (see `VANILLA_BUGS.md` for full details and tests):

- **1/256 miss glitch** — optimal rounding with three-way accuracy logic
- **Bide errors** — accumulated damage clearing fixed for link battles + Bide now misses invulnerable targets (Fly/Dig)
- **Catch rate RNG oversight** — rejection sampling replaced with multiplication-based range reduction
- **Counter glitches** — stale damage cleared on switch-in/battle start, wDamage cleared on can't-move turns, link desync fixed (wUsedMove instead of wSelectedMove)
- **Critical hit ratio error** — Focus Energy / Dire Hit division direction corrected
- **Defrost move forcing** — defrosted Pokémon skips its turn instead of using a stale/wrong move (fixes link desync, PP underflow, wrong move)
- **Division by 0** — defense clamped to minimum 1 after stat scaling to prevent damage calculation freeze
- **Exp. All oversight** — experience distribution fixed
- **Experience underflow** — CalcExperience clamps negative results to 0, preventing Medium Slow level 1 → 100 jump; CalcLevelFromExperience capped at MAX_LEVEL to prevent PC withdrawal freeze
- **Ghost Marowak bypassing** — Poke Doll no longer bypasses ghost Marowak
- **HP recovery move failure** / **Rest remainder oversight** — high-byte comparison fixed
- **Hyper Beam + Freeze permanent helplessness** — Haze freeze / recharge softlock fixed
- **Hyper Beam + Sleep move glitch** — sleep moves no longer bypass accuracy/status checks against recharging targets
- **Index #000 post-capture** — `or 1` ensures capture flag is non-zero for species #000, ending the battle properly
- **Jump Kick / Hi Jump Kick crash damage** — crash damage now correctly 1/8 of would-be damage (was always 1 HP)
- **Level-up learnset skipping** — all moves at or below the new level are now learned, even when skipping intermediate levels
- **Invulnerability glitch** — Fly/Dig invulnerability cleared on full paralysis/confusion
- **Mew glitch** — `BIT_SEEN_BY_TRAINER` cleared on map entry
- **Mimic level-up glitch** — Mimic'd move preserved when learning new moves during level-up
- **Mirror Move glitch** — added Mirror Move check alongside Metronome in trapping move switch handler
- **Psywave desynchronization** — RNG parity fixed for link battles
- **Psywave infinite loop** — upper bound clamped to minimum 2 for levels 0/1/171
- **Red bar glitch** — low HP alarm limited to 4 beep cycles per activation, allowing battle sounds to play
- **Rematching Trainers** — same fix as Mew glitch (BIT_SEEN_BY_TRAINER cleared on map entry)
- **Stat modification errors** — removed stacking badge boosts and wrong-target burn/paralysis penalties from stat modifier functions
- **Struggle bypassing** — PP_MASK guard in DecrementPP prevents PP underflow from auto-selected 0-PP moves
- **Substitute + Confusion glitch** — self-damage paths bypass Substitute check via ApplyDamage*Direct labels
- **Substitute HP drain bug** — Drain/Dream Eater vs Substitute check fixed
- **Super Glitch** — move name lookups clamped to prevent buffer overflow from glitch move IDs
- **Toxic counter glitches** — Leech Seed uses flat maxHP/16 (no Toxic N multiplier); Rest resets BADLY_POISONED and counter
- **Transform glitches** — catch-as-Ditto fixed, SELECT swap fixed, Mirror Move/Metronome PP error fixed
- **Trapping sleep glitch** — sleep/freeze counters decrement even when CANNOT_MOVE (trapped)
- **ZZAZZ glitch** — prize money BCD overflow fixed by reloading DE pointer in ReadTrainer.LastLoop
- **Silent Indigo Plateau** — victory music replayed after evolution during Champion battle
- **Battle draw theme oversight** — victory music suppressed on Explosion/Self-Destruct double faint
- **Dual-type damage misinformation** — effectiveness message corrected for dual-type neutral damage
- **Ghost identity unveiling** — ghost sprite no longer revealed on party menu return
- **0 damage glitch** — 0.25x effective moves clamp to 1 damage instead of missing
- **Pikachu cry in link battles** — enemy Pikachu now uses voice cry instead of electronic cry
- **Inverted sprites** — wSpriteFlipped cleared on invalid dex number path to prevent persistent sprite inversion
- **Link battle animation oversight** — Minimize visual effect applied regardless of animation setting (Gen II fix)
- **Mimic PP glitch** — fight menu max PP now shows Mimic's base PP instead of copied move's
- **Poison/Burn animation with 0 HP** — residual damage animation skipped when HP is already 0

## Gameplay-affecting glitches

### Pokémon Red, Green, Blue, and Yellow

#### 0 damage glitch

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "0 damage glitch (0.25x effective move misses instead of dealing 1)"

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium and English Stadium? |
| --- | --- |

If a damaging [move](https://bulbapedia.bulbagarden.net/wiki/Move "Move") 's [damage](https://bulbapedia.bulbagarden.net/wiki/Damage "Damage") calculation yields 0 if it hits a Pokémon whose both [types](https://bulbapedia.bulbagarden.net/wiki/Type "Type") resist the move's type, the move will instead miss as if it were ineffective.

By [Crystal\_](https://www.youtube.com/channel/UCQcizw_rc-q55lmwU3w6-wA)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=fxNzPeLlPTU).**

#### 1/256 miss glitch

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "1/256 miss glitch"

In the [Generation I](https://bulbapedia.bulbagarden.net/wiki/Generation_I "Generation I") handheld games and [Pocket Monsters Stadium](https://bulbapedia.bulbagarden.net/wiki/Pocket_Monsters_Stadium "Pocket Monsters Stadium"), all moves are 1/256 more likely to miss than was intended, including 100% [accuracy](https://bulbapedia.bulbagarden.net/wiki/Accuracy "Accuracy") moves. In non-Japanese versions, [Swift](https://bulbapedia.bulbagarden.net/wiki/Swift_\(move\) "Swift (move)") and [Bide](https://bulbapedia.bulbagarden.net/wiki/Bide_\(move\) "Bide (move)") skip accuracy checks and always hit, regardless of this bug.

Moves with 100% accuracy have a 255/256 (~99.6%) chance of hitting (without [accuracy](https://bulbapedia.bulbagarden.net/wiki/Stat#Accuracy "Stat") nor [evasion](https://bulbapedia.bulbagarden.net/wiki/Stat#Evasion "Stat") modifiers). Other moves also have 1/256 less accuracy than was intended.

The glitch occurs due to the accuracy check using a "strictly less than" comparison instead of a "less than or equal to" comparison. If a randomly generated integer between 0 and 255 (inclusive) is strictly less than the move's accuracy (after applying accuracy and evasion modifiers), the move hits; however, if the random number is exactly 255, the random number cannot be less than the move's modified accuracy (regardless of its value).

In [Pokémon Stadium](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium "Pokémon Stadium"), the randomly generated integer is between 0 and 254 (inclusive). This prevents moves with 100% accuracy from missing, but also slightly increases the probability of lower accuracy moves hitting.

In [Pokémon Gold and Silver](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Gold_and_Silver_Versions "Pokémon Gold and Silver Versions"), if the move has 100% accuracy (after applying accuracy and evasion modifiers and the [BrightPowder](https://bulbapedia.bulbagarden.net/wiki/Bright_Powder "Bright Powder") modifier), the move skips the rest of the accuracy check and hits. This prevents moves with 100% accuracy from missing, but does not affect the chance of hitting for moves with lower accuracy.

By [Werster](https://www.youtube.com/channel/@Werster)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=KF6Icb9JYns).**

#### Bide errors

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Bide errors" (accumulated damage clearing for link battles + Bide now misses invulnerable targets using Fly/Dig)

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium? |
| --- | --- |

The stored damage from [Bide](https://bulbapedia.bulbagarden.net/wiki/Bide_\(move\) "Bide (move)") can hit (but not always) a Pokémon under the invulnerable stage of [Fly](https://bulbapedia.bulbagarden.net/wiki/Fly_\(move\) "Fly (move)") or [Dig](https://bulbapedia.bulbagarden.net/wiki/Dig_\(move\) "Dig (move)"). If Bide deals damage to a Pokémon under Fly or Dig, the game will reveal its sprite early. This also causes a small animation glitch with Dig where it appears that the enemy Pokémon rises from the ground off the top of the screen instead of the enemy rising up from 'underground'. There is no animation glitch with [Fly](https://bulbapedia.bulbagarden.net/wiki/Fly_\(move\) "Fly (move)") because the game has no animation on the opponent's side of Fly returning from the top of the screen to the ground—the game only reveals the sprite.

This was fixed in [Pokémon Stadium](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium "Pokémon Stadium").

By [ChickasaurusGL](https://www.youtube.com/channel/UCZz2ixp-5T6VeAPtAMQ5v5Q)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=mpHw7CPQdQY).**

#### Catch rate RNG oversight

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Catch rate RNG oversight" (rejection sampling replaced with multiplication-based range reduction)

Due to the combination of the way the games' random number generator is implemented and the capture algorithm using rejection sampling to generate a random number from a limited range for Great and Ultra/Safari Balls, there is a significant bias to RNG outcomes for these balls; Ultra Balls can for instance be less effective than Poké Balls against Pokémon with high catch rates at full health, and Pokémon with lower catch rates are significantly harder to catch in an Ultra or Safari Ball at full health than they should, while being easier to catch at low health.[^1]

#### Counter glitches

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Counter glitches" (stale damage cleared on switch/battle start, wDamage cleared on can't-move turns, HandleCounterMove checks wUsedMove instead of cursor-polluted wSelectedMove)

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium? |
| --- | --- |

[Counter](https://bulbapedia.bulbagarden.net/wiki/Counter_\(move\) "Counter (move)") may strike back damage from an attack that isn't Normal- or Fighting-type. For this to happen, the Counter target must have not selected any move the turn Counter was used (for example, due to being frozen, asleep, or switching out), and must have moved first and used a Normal- or Fighting-type damaging move the previous turn. In addition, Counter may also strike back damage from one's own attack. This occurs if the Counter target previously used a Normal- or Fighting-type damaging move before the Counter user successfully used any damaging move during the same turn. If, in the next turn when Counter is used, the Counter target doesn't select a move, the Counter user's own damage will be dealt.

In Link Battles, Counter may also trigger desynchronization errors. This occurs due to the last move pointed by the cursor in the move selection menu being treated as the last move actually used if the Pokémon switches out. This oversight can also be exploited outside of link battles to make the opponent's Counter hit or miss at will under specific circumstances.

This was fixed in [Pokémon Stadium](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium "Pokémon Stadium").

By [Crystal\_](https://www.youtube.com/channel/UCQcizw_rc-q55lmwU3w6-wA)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=ftTalHMjPRY).**

#### Critical hit ratio error

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Focus Energy / Dire Hit bug"

[Focus Energy](https://bulbapedia.bulbagarden.net/wiki/Focus_Energy_\(move\) "Focus Energy (move)") and [Dire Hits](https://bulbapedia.bulbagarden.net/wiki/Dire_Hit "Dire Hit") are intended to quadruple the [critical hit](https://bulbapedia.bulbagarden.net/wiki/Critical_hit "Critical hit") rate, but due to a glitch, they will quarter the chance of scoring a critical hit. This was fixed in [Pokémon Stadium](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium "Pokémon Stadium").

#### Defrost move forcing

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Defrost move forcing"

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium and English Stadium? |
| --- | --- |

If a [frozen](https://bulbapedia.bulbagarden.net/wiki/Freeze_\(status_condition\) "Freeze (status condition)") Pokémon is defrosted before it would have moved that turn, it uses a move that turn, even though it couldn't select a move that turn due to being frozen. However, this move can differ between the games in a link battle, causing desynchronization. Additionally, this can also allow a Pokémon to use a move with no [PP](https://bulbapedia.bulbagarden.net/wiki/PP "PP") remaining, causing an underflow.

If a Pokémon is defrosted, in the game of the owner of the defrosted Pokémon, the move used will be the last move the player had the cursor over. Since the player does not get to select a move while frozen, this can be a move of another Pokémon in the party. The value that manages this is set to 0 at the start of a link battle, so if the player has never moved the cursor over a move during that battle, the used move will be the glitch move [\--](https://bulbapedia.bulbagarden.net/wiki/--_\(move\) "-- (move)").

In the game of the other player, the move used will be the last move used by the defrosted Pokémon (reset upon switching), or the first listed move if it has not used a move since switching.

PP is deducted from the move the Pokémon uses in other player's game (even in its owner's game), regardless of its current PP. If the move had 0 PP, it underflows to 63 PP and removes the effect of one [PP Up](https://bulbapedia.bulbagarden.net/wiki/PP_Up "PP Up").

By [Crystal\_](https://www.youtube.com/channel/UCQcizw_rc-q55lmwU3w6-wA)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=iSSf4XaqGAU).**

#### Division by 0

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Division by 0 (damage calculation freeze)"

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium and English Stadium? |
| --- | --- |

During damage calculation, the game will eventually attempt to [divide by 0](https://en.wikipedia.org/wiki/Division_by_zero "wp:Division by zero") in the following two cases. In both cases, this causes the game to [freeze](https://bulbapedia.bulbagarden.net/wiki/Game_freeze#%22Softlocking%22 "Game freeze") indefinitely (due to the algorithm looping infinitely).

The attacker's current Attack/Special stat is higher than 255 and the defender's current Defense/Special stat is lower than 4.

By [Crystal\_](https://www.youtube.com/channel/UCQcizw_rc-q55lmwU3w6-wA)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=V6iUlyS8GMU).**

The defender's current Defense/Special stat is 512 or 513 and the defender has used [Reflect](https://bulbapedia.bulbagarden.net/wiki/Reflect_\(move\) "Reflect (move)") / [Light Screen](https://bulbapedia.bulbagarden.net/wiki/Light_Screen_\(move\) "Light Screen (move)"). In addition, if its current Defense/Special stat is 514 or higher when [Reflect](https://bulbapedia.bulbagarden.net/wiki/Reflect_\(move\) "Reflect (move)") / [Light Screen](https://bulbapedia.bulbagarden.net/wiki/Light_Screen_\(move\) "Light Screen (move)") is up, it will be treated as if it was much lower due to a roll-over glitch.

By [Crystal\_](https://www.youtube.com/channel/UCQcizw_rc-q55lmwU3w6-wA)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=fVtO_DKxIsI).**

#### Exp. All oversight

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Exp. All experience distribution"

*Main article: [Experience#Apparent Exp. All programming error in Generation I](https://bulbapedia.bulbagarden.net/wiki/Experience#Apparent_Exp._All_programming_error_in_Generation_I "Experience")*

If the player has an [Exp. All](https://bulbapedia.bulbagarden.net/wiki/Exp._Share "Exp. Share") in their bag and uses two or more Pokémon from their party in battle, then the total amount of experience and [stat experience](https://bulbapedia.bulbagarden.net/wiki/Effort_values#Stat_experience "Effort values") gained overall will be decreased depending on the number of Pokémon used.

#### Experience underflow

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Experience underflow (Medium Slow level 1 → 100 jump)" and "Experience PC withdrawal freeze (CalcLevelFromExperience MAX_LEVEL cap)"

*Main article: [Experience#Experience underflow glitch](https://bulbapedia.bulbagarden.net/wiki/Experience#Experience_underflow_glitch "Experience")*

In Generations I and II, level 1 Pokémon using the "Medium Slow" growth algorithm will jump from level 1 to level 100 after gaining a low amount of experience points.

By [LunarRay](https://www.youtube.com/channel/UCJ8cXwiP6PH57Ya1i4WKIlg)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=EtkRiiwD0jc).**

#### Ghost Marowak bypassing

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Poke Doll bypasses ghost Marowak battle"

In the [Pokémon Tower](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Tower "Pokémon Tower"), it is possible to cause the [ghost Marowak](https://bulbapedia.bulbagarden.net/wiki/Marowak_\(ghost\) "Marowak (ghost)") to permanently disappear by using a [Poké Doll](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9_Doll "Poké Doll") to end the battle against it. If this is done, there is no need to use the [Silph Scope](https://bulbapedia.bulbagarden.net/wiki/Silph_Scope "Silph Scope") to reveal its appearance.

This also allows the player to [break the gameplay sequence](https://bulbapedia.bulbagarden.net/wiki/Sequence_breaking "Sequence breaking") and obtain the [Poké Flute](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9_Flute "Poké Flute") without entering the [Team Rocket Hideout](https://bulbapedia.bulbagarden.net/wiki/Team_Rocket_Hideout "Team Rocket Hideout") and acquiring the Silph Scope. In the context of [speedrunning](https://bulbapedia.bulbagarden.net/wiki/Speedrun "Speedrun"), this is known as the "Poké Doll skip."

By [Wooggle](https://www.youtube.com/channel/UCgA3xOk7QY4MOYhc7EBFe0g)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=p_muMF45X-4).**

In the [Pokémon Tower](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Tower "Pokémon Tower"), the [individual values](https://bulbapedia.bulbagarden.net/wiki/Individual_values "Individual values") of the [ghost Marowak](https://bulbapedia.bulbagarden.net/wiki/Marowak_\(ghost\) "Marowak (ghost)") are randomly generated like those of any [wild Pokémon](https://bulbapedia.bulbagarden.net/wiki/Wild_Pok%C3%A9mon "Wild Pokémon"). If the player battles it multiple times in the same game, it will have new individual values for each battle despite being the same Pokémon.

#### HP recovery move failure

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Healing moves fail when HP is 255 or 511 below max"

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium? |
| --- | --- |

If a Pokémon uses a recovery move ([Softboiled](https://bulbapedia.bulbagarden.net/wiki/Softboiled_\(move\) "Softboiled (move)"), [Rest](https://bulbapedia.bulbagarden.net/wiki/Rest_\(move\) "Rest (move)") or [Recover](https://bulbapedia.bulbagarden.net/wiki/Recover_\(move\) "Recover (move)")) and the difference between its current HP and maximum HP is 255 or 511 (or any number that leaves a remainder of 255 when divided by 256), the move will fail the same way it would when the difference is 0. This glitch does not occur in Pokémon Stadium.

This is caused by the comparison that checks whether the current HP matches the maximum HP erroneously not correctly incorporating the upper byte of the HP values.

By [ChickasaurusGL](https://www.youtube.com/channel/UCZz2ixp-5T6VeAPtAMQ5v5Q)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=sqkBby1HlmY).**

#### Hyper Beam + Freeze permanent helplessness

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Haze freeze / Hyper Beam recharge softlock"

If a Pokémon uses [Hyper Beam](https://bulbapedia.bulbagarden.net/wiki/Hyper_Beam_\(move\) "Hyper Beam (move)") and then becomes frozen before it's set to recharge at its following turn, the Hyper Beam user will be stuck permanently in a state of waiting to recharge, and cannot switch out or select any moves until it faints, or thaws from a [Fire](https://bulbapedia.bulbagarden.net/wiki/Fire_\(type\) "Fire (type)") move. This glitch was fixed in all versions of [Pokémon Stadium](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium "Pokémon Stadium").

By [SadisticMystic](https://www.youtube.com/channel/UC3Lha-y4n1fFoTyqefHo1Cg)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=HfHSQ_XbfAk).**

If the Pokémon thaws as a result of an opponent's [Haze](https://bulbapedia.bulbagarden.net/wiki/Haze_\(move\) "Haze (move)") instead of a Fire move, the Pokémon will remain subject to this glitch, and subsequent use of a Fire move will not have any effect on its status either.

By [ChickasaurusGL](https://www.youtube.com/channel/UCZz2ixp-5T6VeAPtAMQ5v5Q)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=FjZreYA2m_w).**

#### Hyper Beam + Sleep move glitch

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Hyper Beam + Sleep move glitch"

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium? |
| --- | --- |

If a Pokémon uses [Hyper Beam](https://bulbapedia.bulbagarden.net/wiki/Hyper_Beam_\(move\) "Hyper Beam (move)") and needs to recharge, if it is affected by a sleep-inducing move, any other status it may already have ([paralysis](https://bulbapedia.bulbagarden.net/wiki/Paralysis_\(status_condition\) "Paralysis (status condition)"), [burn](https://bulbapedia.bulbagarden.net/wiki/Burn_\(status_condition\) "Burn (status condition)"), [poison](https://bulbapedia.bulbagarden.net/wiki/Poison_\(status_condition\) "Poison (status condition)"), or [freeze](https://bulbapedia.bulbagarden.net/wiki/Freeze_\(status_condition\) "Freeze (status condition)")) will be ignored and sleep will be induced regardless. In addition, the sleep-inducing move will never miss, as it will skip any accuracy checks in a similar way to [Swift](https://bulbapedia.bulbagarden.net/wiki/Swift_\(move\) "Swift (move)").

Furthermore, if the Pokémon was badly poisoned (by [Toxic](https://bulbapedia.bulbagarden.net/wiki/Toxic_\(move\) "Toxic (move)")), [the Toxic counter will not be reset](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Toxic_counter_glitches).

By [Crystal\_](https://www.youtube.com/channel/UCQcizw_rc-q55lmwU3w6-wA)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=x2AgAdQwyGI).**

#### Index #000 post-capture

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Index #000 post-capture (battle continues after catching 'M)"

In [Generation I](https://bulbapedia.bulbagarden.net/wiki/Generation_I "Generation I"), if the player manages to capture an ['M (00)](https://bulbapedia.bulbagarden.net/wiki/%27M_\(00\) "'M (00)") or [3TrainerPoké $](https://bulbapedia.bulbagarden.net/wiki/3TrainerPok%C3%A9_$ "3TrainerPoké $") an invisible wild [Ditto](https://bulbapedia.bulbagarden.net/wiki/Ditto_\(Pok%C3%A9mon\) "Ditto (Pokémon)") will still be in battle with the player where 'M (00) was before, and the battle will not end. This Ditto can then be caught.

By [pandakekok](https://www.youtube.com/pandakekok)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=x_TS7pVybKg).**

#### Invulnerability glitch

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Fly/Dig invulnerability persists through full paralysis"

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium? |
| --- | --- |

In [Generation I](https://bulbapedia.bulbagarden.net/wiki/Generation_I "Generation I"), if a Pokémon is fully [paralysed](https://bulbapedia.bulbagarden.net/wiki/Paralysis_\(status_condition\) "Paralysis (status condition)") or hurts itself in [confusion](https://bulbapedia.bulbagarden.net/wiki/Confusion_\(status_condition\) "Confusion (status condition)") while in the semi-invulnerable stage of [Fly](https://bulbapedia.bulbagarden.net/wiki/Fly_\(move\) "Fly (move)") or [Dig](https://bulbapedia.bulbagarden.net/wiki/Dig_\(move\) "Dig (move)"), all moves (with the exception of [Swift](https://bulbapedia.bulbagarden.net/wiki/Swift_\(move\) "Swift (move)"), [Transform](https://bulbapedia.bulbagarden.net/wiki/Transform_\(move\) "Transform (move)") and possibly the unleashed damage from [Bide](https://bulbapedia.bulbagarden.net/wiki/Bide_\(move\) "Bide (move)")) from the opponent will miss or fail until the user switches Pokémon, finishes the battle or successfully performs a [charging move](https://bulbapedia.bulbagarden.net/wiki/Category:Moves_with_a_charging_turn "Category:Moves with a charging turn") (specifically, Fly, Dig, [Razor Wind](https://bulbapedia.bulbagarden.net/wiki/Razor_Wind_\(move\) "Razor Wind (move)"), [Skull Bash](https://bulbapedia.bulbagarden.net/wiki/Skull_Bash_\(move\) "Skull Bash (move)"), and [Solar Beam](https://bulbapedia.bulbagarden.net/wiki/Solar_Beam_\(move\) "Solar Beam (move)")). The user's Pokémon can attack normally during this glitch.

This glitch was fixed in [Pokémon Stadium](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium "Pokémon Stadium").

By [ChickasaurusGL](https://www.youtube.com/channel/UCZz2ixp-5T6VeAPtAMQ5v5Q)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=bNzDmXbZ7kY).**

#### Jump Kick and Hi Jump Kick's crash damage

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Jump Kick / Hi Jump Kick crash damage (always 1 HP instead of damage/8)"

[Jump Kick](https://bulbapedia.bulbagarden.net/wiki/Jump_Kick "Jump Kick") and [Hi Jump Kick](https://bulbapedia.bulbagarden.net/wiki/Hi_Jump_Kick "Hi Jump Kick") deal [crash damage](https://bulbapedia.bulbagarden.net/wiki/Crash_damage "Crash damage") of exactly 1 HP to the user if the move misses. This is also the case in [Pokémon Stadium](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium "Pokémon Stadium"), even though the description of Hi Jump Kick falsely states that the crash damage is 1/8 of the damage it would have caused.

In [Generation II](https://bulbapedia.bulbagarden.net/wiki/Generation_II "Generation II"), the crash damage of both moves is 1/8 of the damage it would have caused, as previously stated in Stadium.

#### Level-up learnset skipping

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Level-up learnset skipping (moves missed when skipping levels)"

Pokémon cannot learn moves they should learn at a level if they earn enough experience at once to skip that level.

For example, if a level 4 [Pidgey](https://bulbapedia.bulbagarden.net/wiki/Pidgey_\(Pok%C3%A9mon\) "Pidgey (Pokémon)") earned enough experience points for defeating a single Pokémon to reach level 6 or higher, it will not learn [Sand-Attack](https://bulbapedia.bulbagarden.net/wiki/Sand-Attack_\(move\) "Sand-Attack (move)"), a move it would normally learn at level 5.

In the [Pokémon Stadium series](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium_series "Pokémon Stadium series"), this glitch is not present because Pokémon do not level up in battle.

By [Wooggle](https://www.youtube.com/channel/UCgA3xOk7QY4MOYhc7EBFe0g)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=Fvn7xHxb6BU).**

#### Mew glitch

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Trainer Fly / Mew glitch (stale BIT_SEEN_BY_TRAINER on map entry)"

*Main article: [Mew glitch](https://bulbapedia.bulbagarden.net/wiki/Mew_glitch "Mew glitch")*

#### Mimic level up glitch

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Mimic level-up glitch (Mimic'd move reverts on learning new move)"

In this generation only, if a Pokémon that used Mimic levels up in battle and learns a new [move](https://bulbapedia.bulbagarden.net/wiki/Move "Move"), Mimic's effect is reverted. The move copied by Mimic is lost, and Mimic will be usable again.

In the [Pokémon Stadium series](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium_series "Pokémon Stadium series"), this glitch is not present because Pokémon do not level up in battle.

#### Mirror Move glitch

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Mirror Move link battle desync with trapping moves"

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium and English Stadium? |
| --- | --- |

During a Link Battle, if [Mirror Move](https://bulbapedia.bulbagarden.net/wiki/Mirror_Move_\(move\) "Mirror Move (move)") and a [binding move](https://bulbapedia.bulbagarden.net/wiki/Category:Binding_moves "Category:Binding moves") (such as [Wrap](https://bulbapedia.bulbagarden.net/wiki/Wrap_\(move\) "Wrap (move)") or [Fire Spin](https://bulbapedia.bulbagarden.net/wiki/Fire_Spin_\(move\) "Fire Spin (move)")) are used together, the two player's games may become desynchronized due to one game interpreting that the attack used was Mirror Move (and failing) and the other game interpreting that the binding move was used instead.

By [SloshedMail](https://www.youtube.com/channel/UCqnd5LOJjH5SURgI2RuYFBg)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=O8GMyy7x3WE&NR).**

#### Psywave desynchronization

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Psywave link battle desync"

If a Pokémon uses [Psywave](https://bulbapedia.bulbagarden.net/wiki/Psywave_\(move\) "Psywave (move)") in a link battle, there is a small chance the games will generate a different number of pseudo-random numbers, causing desynchronization.

When a Pokémon uses Psywave, a random number is generated between 0 and 255. If the player uses Psywave, if the generated number is greater than or equal to 1.5× the Pokémon's level (rounded down) or it is 0, the number is discarded and a new number generated; if the opponent uses Psywave, if the generated number is greater than or equal to the Pokémon's level, the number is discarded and a new number generated. As such, if the generated number is 0, the Psywave user's game will generate a new number, whereas the non-Psywave user's game will not. This causes all subsequent pseudo-random numbers to be desynchronized.

By [Crystal\_](https://www.youtube.com/channel/UCQcizw_rc-q55lmwU3w6-wA)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=5KmTCdnWzVI).**

#### Psywave infinite loop

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Psywave infinite loop (levels 0, 1, and 171)"

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium and English Stadium? |
| --- | --- |

If a level 0, 1 or 171 Pokémon uses [Psywave](https://bulbapedia.bulbagarden.net/wiki/Psywave_\(move\) "Psywave (move)"), the game will continuously generate pseudo-random numbers, [hanging indefinitely](https://bulbapedia.bulbagarden.net/wiki/Softlock "Softlock"). However, this is not an issue in normal gameplay, as Pokémon can only be obtained at these levels via glitches.

When a Pokémon uses Psywave, a random number is generated between 0 and 255. If the generated number is greater than or equal to 1.5× the Pokémon's level (rounded down) or it is 0, the number is discarded and a new number generated. As such, there is no number that can be generated for a level 0 or level 1 Pokémon that will not result in the result being discarded and a new number being generated.

If a level 171 Pokémon uses Psywave, the upper bound would be 256; however, since this value is stored in a single byte, it overflows to 0, causing the same issue as a level 0 Pokémon.

By [Crystal\_](https://www.youtube.com/channel/UCQcizw_rc-q55lmwU3w6-wA)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=VyIFL_-l2o4).**

#### Red bar glitch

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Red bar glitch (low HP alarm overrides battle sounds)"

If a Pokémon is in a critical health status, the game will start to play a sound to let the player know their Pokémon is low on health and about to possibly faint. This sound prevents other sounds and animations from playing due to the limited number of audio channels in the Game Boy's hardware. This glitch has become very well known and is often used in [speedruns](https://bulbapedia.bulbagarden.net/wiki/Speedrun "Speedrun") for the Generation I games.

#### Rematching Trainers

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Trainer Fly / Mew glitch (stale BIT_SEEN_BY_TRAINER on map entry)" (same underlying cause)

To perform this glitch, the player must have some Pokémon which can lose a battle easily, so they may wish to have one Pokémon, a [poisoned](https://bulbapedia.bulbagarden.net/wiki/Poison_\(status_condition\) "Poison (status condition)") Pokémon or both. They must have access to an unbattled Trainer who is inside a [cave](https://bulbapedia.bulbagarden.net/wiki/Cave "Cave"). (e.g. [Mt. Moon](https://bulbapedia.bulbagarden.net/wiki/Mt._Moon "Mt. Moon") or [Victory Road](https://bulbapedia.bulbagarden.net/wiki/Victory_Road_\(Kanto\) "Victory Road (Kanto)")). They must encounter a wild Pokémon while in a Trainer's eyeline. This wild Pokémon must proceed to defeat the player, sending them to a [Pokémon Center](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Center "Pokémon Center"). They then must re-enter the cave the Trainer is in. The Start menu will pop up. Upon closing it, the Trainer they escaped from will fight the player. However, if the player defeats them, this is not interpreted as beating the Trainer, and the player can challenge them again.

#### Rest remainder oversight

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Healing moves fail when HP is 255 or 511 below max" (same underlying bug as HP recovery move failure)

In this generation, [Rest](https://bulbapedia.bulbagarden.net/wiki/Rest_\(move\) "Rest (move)") will fail if the difference between the user's maximum HP and current HP leaves a remainder of 255 when divided by 256 (such as 255 or 511).

#### Stat modification errors

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Stat modification errors (badge boost stacking, wrong-target status penalties)"

Every time a Pokémon successfully uses a move that affects a stat stage (either raising for example with [Double Team](https://bulbapedia.bulbagarden.net/wiki/Double_Team_\(move\) "Double Team (move)"), or lowering it for example with [Screech](https://bulbapedia.bulbagarden.net/wiki/Screech_\(move\) "Screech (move)")) of any of the two Pokémon in battle, the following happens:

- The stat in question is recalculated from its out-of-battle stat and stat stage.
- If the target was the player's Pokémon, [badge](https://bulbapedia.bulbagarden.net/wiki/Badge "Badge") boosts are applied to all of its stats (if the player has the corresponding badge), boosting them by 1/8.
- If the Pokémon whose turn it is is not paralyzed, its current Speed stat gets quartered.
- If the Pokémon whose turn it is is not burned, its current Attack stat gets halved.

This leads to three notable unintended stat-related effects:

- Whenever one of the player's Pokémon's stat stages is modified, all of its other badge-boosted stats are multiplied by 1.125 again, even though they were already boosted. (The affected stat is recalculated correctly.) This effect can stack until a stat reaches the maximum of 999.
- When a Pokémon is burned or paralyzed, the Attack/Speed drop from the status will be applied again whenever its opponent uses a move that modifies stat stages (either raising its own or lowering the enemy's). This will similarly stack until the stat in question has dropped to the minimum of 1.
- When a burned or paralyzed Pokémon raises its Attack or Speed stat respectively with moves such as Swords Dance or Agility, the stat is recalculated in accordance with the boosted stat stage, but the status drop is not applied to it afterwards (since it's erroneously applied to the wrong Pokémon), effectively nullifying its effects.

All of these issues were fixed in the [Pokémon Stadium series](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium_series "Pokémon Stadium series").

By [Crystal\_](https://www.youtube.com/channel/UCQcizw_rc-q55lmwU3w6-wA)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=GlhsYKeUt-w).**

#### Struggle bypassing

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Struggle bypassing PP underflow"

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium and English Stadium? |
| --- | --- |

In Generation I, a Pokémon can avoid using [Struggle](https://bulbapedia.bulbagarden.net/wiki/Struggle_\(move\) "Struggle (move)") by allowing the game to self-select a move to be used, which can happen to any move used immediately after a Pokémon is thawed out after being [frozen](https://bulbapedia.bulbagarden.net/wiki/Freeze_\(status_condition\) "Freeze (status condition)"), or due to the effects of one of several moves ([Bind](https://bulbapedia.bulbagarden.net/wiki/Bind_\(move\) "Bind (move)"), [Clamp](https://bulbapedia.bulbagarden.net/wiki/Clamp_\(move\) "Clamp (move)"), [Fire Spin](https://bulbapedia.bulbagarden.net/wiki/Fire_Spin_\(move\) "Fire Spin (move)"), [Hyper Beam](https://bulbapedia.bulbagarden.net/wiki/Hyper_Beam_\(move\) "Hyper Beam (move)"), [Metronome](https://bulbapedia.bulbagarden.net/wiki/Metronome_\(move\) "Metronome (move)"), [Mimic](https://bulbapedia.bulbagarden.net/wiki/Mimic_\(move\) "Mimic (move)"), and [Wrap](https://bulbapedia.bulbagarden.net/wiki/Wrap_\(move\) "Wrap (move)")) because of the auto-selection involved with [binding moves](https://bulbapedia.bulbagarden.net/wiki/Category:Binding_moves "Category:Binding moves"). A move used with 0 [PP](https://bulbapedia.bulbagarden.net/wiki/PP "PP") in this way [underflows](https://en.wikipedia.org/wiki/Arithmetic_underflow "wp:Arithmetic underflow") to the maximum possible value, 63 PP; due to the way the data is structured, if this occurs, a move on which 0 [PP Ups](https://bulbapedia.bulbagarden.net/wiki/PP_Up "PP Up") had been used will gain full PP Up status, while a move on which PP Ups had been used loses one PP Up boost.

From Generation II onward, this bug is addressed by preventing a move from being executed if it has 0 PP.

By [ChickasaurusGL](https://www.youtube.com/channel/UCZz2ixp-5T6VeAPtAMQ5v5Q)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=I8AzgGoJbTs).**

#### Substitute HP drain bug

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Drain/Dream Eater vs Substitute check fixed"

In the Western versions of [Pokémon Red, Blue](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Red_and_Blue_Versions "Pokémon Red and Blue Versions"), and [Yellow](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Yellow_Version "Pokémon Yellow Version"), HP-draining moves can hit a substitute (like any other move) due to a programming oversight.

In [Pokémon Gold, Silver](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Gold_and_Silver_Versions "Pokémon Gold and Silver Versions"), and [Crystal](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Crystal_Version "Pokémon Crystal Version"), the Japanese versions of [Pokémon Red, Green](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Red_and_Green_Versions "Pokémon Red and Green Versions"), [Blue](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Blue_Version_\(Japanese\) "Pokémon Blue Version (Japanese)"), and [Yellow](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Yellow_Version "Pokémon Yellow Version"), and the [Pokémon Stadium series](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium_series "Pokémon Stadium series"), [HP](https://bulbapedia.bulbagarden.net/wiki/HP "HP") -draining [moves](https://bulbapedia.bulbagarden.net/wiki/Move "Move") always miss when used against a target that is behind a [substitute](https://bulbapedia.bulbagarden.net/wiki/Substitute_\(doll\) "Substitute (doll)").

By [ChickasaurusGL](https://www.youtube.com/ChickasaurusGL)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=Rrn4rtQXYQ0).**

#### Substitute + Confusion glitch

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Substitute + Confusion/Jump Kick self-damage glitch"

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium and English Stadium? |
| --- | --- |

If a Pokémon with a Substitute up hurts itself due to confusion, or due to [Jump Kick](https://bulbapedia.bulbagarden.net/wiki/Jump_Kick_\(move\) "Jump Kick (move)") 's or [Hi Jump Kick](https://bulbapedia.bulbagarden.net/wiki/Hi_Jump_Kick_\(move\) "Hi Jump Kick (move)") 's side effect, damage will be dealt to the opponent's Substitute instead. If the opponent doesn't have a Substitute up no damage will be dealt to any Pokémon.

By [Crystal\_](https://www.youtube.com/channel/UCQcizw_rc-q55lmwU3w6-wA)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=jw24URgBi5o).**

#### Super Glitch

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Super Glitch (move name buffer overflow)"

*Main article: [Super Glitch](https://bulbapedia.bulbagarden.net/wiki/Super_Glitch "Super Glitch")*

#### Toxic counter glitches

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Toxic counter glitches (Leech Seed + Rest)"

##### With Leech Seed

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium and English Stadium? |
| --- | --- |

If the target of Leech Seed is also under the effect of [Toxic](https://bulbapedia.bulbagarden.net/wiki/Toxic_\(move\) "Toxic (move)") (or was under that effect and healed itself with [Rest](https://bulbapedia.bulbagarden.net/wiki/Rest_\(move\) "Rest (move)")), because Leech Seed and Toxic both use the same damage algorithm, Leech Seed damage will be affected by Toxic's **N** parameter, and will increase each turn. This does not occur in [Generation II](https://bulbapedia.bulbagarden.net/wiki/Generation_II "Generation II") onward.

By [ChickasaurusGL](https://www.youtube.com/channel/UCZz2ixp-5T6VeAPtAMQ5v5Q)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=bNjEFgsIIIY).**

##### With Rest

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium? |
| --- | --- |

If a Pokémon [badly poisoned](https://bulbapedia.bulbagarden.net/wiki/Poison_\(status_condition\) "Poison (status condition)") by [Toxic](https://bulbapedia.bulbagarden.net/wiki/Toxic_\(move\) "Toxic (move)") uses [Rest](https://bulbapedia.bulbagarden.net/wiki/Rest_\(move\) "Rest (move)"), the Toxic counter will remain, with the **N** value not being reset. If a Pokémon is then poisoned, [burned](https://bulbapedia.bulbagarden.net/wiki/Burn_\(status_condition\) "Burn (status condition)"), or affected by Leech Seed, the damage will draw upon (and increment) the **N** value, and will increase each turn. This does not occur in [Pokémon Stadium](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium "Pokémon Stadium") or [Generation II](https://bulbapedia.bulbagarden.net/wiki/Generation_II "Generation II") onward.

By [Crystal\_](https://www.youtube.com/channel/UCQcizw_rc-q55lmwU3w6-wA)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=4LpWNnfk6tA).**

#### Transform glitches

**Partially fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Transformed Pokemon assumed to be Ditto when catching" (catch-as-Ditto fixed), "Transform move swap protection" (SELECT swap fixed), "Transform + Mirror Move/Metronome PP error" (party PP corruption fixed)

*Main article: [List of Transform glitches](https://bulbapedia.bulbagarden.net/wiki/List_of_Transform_glitches "List of Transform glitches")*

#### Trapping sleep glitch

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Trapping sleep glitch"

To perform this glitch, player's Pokémon has to be [bound](https://bulbapedia.bulbagarden.net/wiki/Bound "Bound") by another Pokémon. Because it is bound, the player should use healing items until the binding ends. If the opposing Pokémon then puts player's Pokémon to sleep on the turn the binding ends, the player's Pokémon will never move. To fix the glitch, the player has to cure the sleep status.

By [ChickasaurusGL](https://www.youtube.com/channel/UCZz2ixp-5T6VeAPtAMQ5v5Q)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=fQF5Z5znLnc).**

#### ZZAZZ

> **Fixed in this ROM hack.** `ReadTrainer.LastLoop` now uses `ld de, wAmountMoneyWon + 2` instead of `inc de / inc de` to reload the BCD write pointer after each `AddBCD` call. This prevents the DE pointer from drifting past `wAmountMoneyWon` on overflow, eliminating the $99 RAM spray that causes the ZZAZZ corruption. See also: [Glitch City Wiki — ZZAZZ glitch](https://glitchcity.wiki/wiki/ZZAZZ_glitch).

*Main article: [ZZAZZ glitch](https://bulbapedia.bulbagarden.net/wiki/ZZAZZ_glitch "ZZAZZ glitch")*

### Pokémon Red, Green, Blue, and Yellow (Japanese)

#### Swift effect glitch

[Swift](https://bulbapedia.bulbagarden.net/wiki/Swift_\(move\) "Swift (move)") was programmed to never miss, but due to a programming error in Pokémon Red, Green and Blue (as well as all known revisions of Japanese Yellow), the move is capable of missing under certain circumstances (i.e. if the foe has raised evasion, or is under the invulnerable stage of [Fly](https://bulbapedia.bulbagarden.net/wiki/Fly_\(move\) "Fly (move)") or [Dig](https://bulbapedia.bulbagarden.net/wiki/Dig_\(move\) "Dig (move)") or possibly from 1/256 miss chance that affects other 100% accuracy moves) unless the foe has put up a [substitute](https://bulbapedia.bulbagarden.net/wiki/Substitute_\(doll\) "Substitute (doll)").

This was amended in the English versions, which have Swift never miss (including when a Pokémon is under the invulnerable stage of Fly or Dig) regardless of whether the foe has set up a Substitute or not.

By [ChickasaurusGL](https://www.youtube.com/channel/UCZz2ixp-5T6VeAPtAMQ5v5Q)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=C6Hkos4vdsU).**

### Pokémon Red, Green, and Blue

#### Evolution stone bypassing

| ![](https://archives.bulbagarden.net/media/upload/thumb/8/8f/BoEANSprite.png/50px-BoEANSprite.png) | This glitch is in need of research.   **Reason:** *Glitch Pokémon which evolve by items*   *You can [discuss this on the talk page](https://bulbapedia.bulbagarden.net/wiki/Talk:List_of_battle_glitches_in_Generation_I).* |
| --- | --- |

[Pokémon that evolve by Evolution stone](https://bulbapedia.bulbagarden.net/wiki/Category:Pok%C3%A9mon_that_evolve_by_Evolution_stone "Category:Pokémon that evolve by Evolution stone") can be evolved without the use of an [Evolution stone](https://bulbapedia.bulbagarden.net/wiki/Evolution_stone "Evolution stone") after a [battle](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_battle "Pokémon battle"). If the Pokémon has [leveled](https://bulbapedia.bulbagarden.net/wiki/Level "Level") up during the battle, and the battle has been finished with another [Pokémon species](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_\(species\) "Pokémon (species)") whose [index number](https://bulbapedia.bulbagarden.net/wiki/List_of_Pok%C3%A9mon_by_index_number_in_Generation_I "List of Pokémon by index number in Generation I") corresponds to the [index number](https://bulbapedia.bulbagarden.net/wiki/List_of_items_by_index_number_in_Generation_I "List of items by index number in Generation I") of the Evolution stone that causes the [Evolution](https://bulbapedia.bulbagarden.net/wiki/Evolution "Evolution"), the game will erroneously begin the Evolution as if the Pokémon evolved after leveling up.

| [Stone](https://bulbapedia.bulbagarden.net/wiki/Evolution_stone "Evolution stone") | Pokémon |
| --- | --- |
| [Moon Stone](https://bulbapedia.bulbagarden.net/wiki/Moon_Stone "Moon Stone") | [Exeggutor](https://bulbapedia.bulbagarden.net/wiki/Exeggutor_\(Pok%C3%A9mon\) "Exeggutor (Pokémon)") |
| [Fire Stone](https://bulbapedia.bulbagarden.net/wiki/Fire_Stone "Fire Stone") | [MissingNo.](https://bulbapedia.bulbagarden.net/wiki/MissingNo. "MissingNo.") (0x20) |
| [Thunderstone](https://bulbapedia.bulbagarden.net/wiki/Thunder_Stone "Thunder Stone") | [Growlithe](https://bulbapedia.bulbagarden.net/wiki/Growlithe_\(Pok%C3%A9mon\) "Growlithe (Pokémon)") |
| [Water Stone](https://bulbapedia.bulbagarden.net/wiki/Water_Stone "Water Stone") | [Onix](https://bulbapedia.bulbagarden.net/wiki/Onix_\(Pok%C3%A9mon\) "Onix (Pokémon)") |
| [Leaf Stone](https://bulbapedia.bulbagarden.net/wiki/Leaf_Stone "Leaf Stone") | [Psyduck](https://bulbapedia.bulbagarden.net/wiki/Psyduck_\(Pok%C3%A9mon\) "Psyduck (Pokémon)") |

Some [glitch Pokémon](https://bulbapedia.bulbagarden.net/wiki/Glitch_Pok%C3%A9mon "Glitch Pokémon") with unusual Evolution flags may evolve this way according to the game 'after exposure to an item', which is not necessarily an Evolution stone.

By [Wooggle](https://www.youtube.com/channel/UCgA3xOk7QY4MOYhc7EBFe0g)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=C3H-zaU6GPs).**

### Pokémon Red and Green (Japanese)

#### Binding move wrong side fainting glitch

This glitch was introduced in the later revision (v1.1) of Red and Green and seemingly does not occur in the v1.0 release. It was fixed in the English Red and Blue.

When the player's Pokémon is immobile due to being [bound](https://bulbapedia.bulbagarden.net/wiki/Bound "Bound") by the opponent's [binding move](https://bulbapedia.bulbagarden.net/wiki/Category:Binding_moves "Category:Binding moves"), but the opponent faints due to [burn](https://bulbapedia.bulbagarden.net/wiki/Burn_\(status_condition\) "Burn (status condition)") or [poison](https://bulbapedia.bulbagarden.net/wiki/Poison_\(status_condition\) "Poison (status condition)"), then the player's Pokémon will supposedly faint in addition to the opponent's Pokémon. This does not occur if the player's Pokémon is the one using the binding move. Despite the player's Pokémon supposedly fainting, its HP is not set to zero.

In a link battle, because the Pokémon is only considered to faint on its Trainer's side of the link and not its opponent's side, it can cause a communication error.

By [ChickasaurusGL](https://www.youtube.com/channel/UCZz2ixp-5T6VeAPtAMQ5v5Q)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=mJUOy-c4cCQ).**

#### Fainted lead experience oversight

| ![](https://archives.bulbagarden.net/media/upload/thumb/8/8f/BoEANSprite.png/50px-BoEANSprite.png) | This glitch is in need of research.   **Reason:** *Multiple fainted lead Pokémon, other versions*   *You can [discuss this on the talk page](https://bulbapedia.bulbagarden.net/wiki/Talk:List_of_battle_glitches_in_Generation_I).* |
| --- | --- |

Before starting a Trainer battle, if the lead user Pokémon is fainted, the Pokémon sent out following it will receive half experience, even though the fainted Pokémon did not participate.

By [ChickasaurusGL](https://www.youtube.com/channel/UCZz2ixp-5T6VeAPtAMQ5v5Q)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=Sy1_Sgbhxu4).**

#### Saffron Gym glitches

Losing to [Sabrina](https://bulbapedia.bulbagarden.net/wiki/Sabrina "Sabrina") in the original versions and returning to the Saffron Gym, will cause the player to receive the post-victory text, TM46 and the Marsh Badge.

By [ChickasaurusGL](https://www.youtube.com/channel/UCZz2ixp-5T6VeAPtAMQ5v5Q)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=EF3RlidjDJk).**

After the battle, if the player exits the last text box before returning to the overworld with B, and immediately holds A, Sabrina's before battle text will run again, allowing the player to rematch Sabrina indefinitely.

By [Exarion](https://www.youtube.com/channel/UC2BX5JgTuHBF1xyn9fPfzJA)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=_XTVUo9kWKw).**

### Pokémon Red and Blue (English)

#### Fight Safari Zone Pokémon trick

*Main article: [Fight Safari Zone Pokémon trick](https://bulbapedia.bulbagarden.net/wiki/Fight_Safari_Zone_Pok%C3%A9mon_trick "Fight Safari Zone Pokémon trick")*

#### Old man glitch

*Main article: [Old man glitch](https://bulbapedia.bulbagarden.net/wiki/Old_man_glitch "Old man glitch")*

### Pokémon Stadium

#### Leech Seed + Toxic

[Leech Seed](https://bulbapedia.bulbagarden.net/wiki/Leech_Seed_\(move\) "Leech Seed (move)") still stacks with [Toxic](https://bulbapedia.bulbagarden.net/wiki/Toxic_\(move\) "Toxic (move)") in [Pokémon Stadium](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium "Pokémon Stadium"). This was fixed from Generation II onwards.

By [MazterP28](https://www.youtube.com/channel/@MasterP28)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=qET1p-VPVlM).**

#### Sleep and Hyper Beam recharge glitch

Just like in the Generation I handheld games, if the opponent uses Hyper Beam and has to recharge, but then gets put to sleep, the sleep-inflicting move will always hit, regardless of its accuracy. If it has a status problem, it will be replaced by the Sleep status instead. This was fixed from [Generation II](https://bulbapedia.bulbagarden.net/wiki/Generation_II "Generation II") onwards.

By [froggy0025](https://www.youtube.com/channel/@froggy0025)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=GFzXpyWeTM4).**

## Audio quirks

These are audio quirks that generally do not affect gameplay.

### Pokémon Red, Green, Blue, and Yellow

#### Battle draw theme oversight

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Victory music plays on Explosion/Self-Destruct double faint"

In this generation only, if the player ends the battle in a draw with [Self-Destruct](https://bulbapedia.bulbagarden.net/wiki/Self-Destruct_\(move\) "Self-Destruct (move)") or [Explosion](https://bulbapedia.bulbagarden.net/wiki/Explosion_\(move\) "Explosion (move)") (knocking out both their last Pokémon and the opposing Pokémon with the same move), the victory theme will play even though the player will [black out](https://bulbapedia.bulbagarden.net/wiki/Black_out "Black out"). Even the message "<Pokémon\> fainted!" does not show up before "<Player\> is out of useable Pok é mon!"

If the opposing Pokémon ends the battle in a draw using one of these moves, the victory theme will not play.

By [ChickasaurusGL](https://www.youtube.com/channel/UCZz2ixp-5T6VeAPtAMQ5v5Q)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=HDWoG2BCGbU).**

#### Silent Indigo Plateau

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Silent Indigo Plateau (evolution kills victory music)"

In the battle against [Blue](https://bulbapedia.bulbagarden.net/wiki/Blue_\(game\) "Blue (game)") at [Indigo Plateau](https://bulbapedia.bulbagarden.net/wiki/Indigo_Plateau "Indigo Plateau"), if the player [evolves](https://bulbapedia.bulbagarden.net/wiki/Evolution "Evolution") a Pokémon in battle and defeats Blue, the music will be muted until [Professor Oak](https://bulbapedia.bulbagarden.net/wiki/Professor_Oak "Professor Oak") comes to congratulate the player.

By [ChickasaurusGL](https://www.youtube.com/channel/UCZz2ixp-5T6VeAPtAMQ5v5Q)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=KxMstD8iWNM).**

### Pokémon Yellow

#### Pikachu cry in link battles

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Pikachu cry in link battles (electronic cry instead of voice)"

In link battles, the partner [Pikachu](https://bulbapedia.bulbagarden.net/wiki/Pikachu_\(Yellow\) "Pikachu (Yellow)") 's [cry](https://bulbapedia.bulbagarden.net/wiki/Cry "Cry") is not consistent between the two games. In its original game, Pikachu says its own name, while it utters an electronic noise instead in the other player's game.

This is a result of Pikachu being treated as a regular Pokémon in the other player's game, instead of having its own cry. This glitch occurs even if both players are playing Pokémon Yellow.

See also [a related glitch](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Pikachu_entering_link_battles) about the Pikachu's animation when entering battles.

## Graphical quirks

These are graphical quirks that appear in battles but generally do not affect gameplay.

### Pokémon Red, Green, Blue, and Yellow

#### Dual-type damage misinformation

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Dual-type move effectiveness message misreported"

*Main article: [Dual-type damage misinformation](https://bulbapedia.bulbagarden.net/wiki/Dual-type_damage_misinformation "Dual-type damage misinformation")*

In this generation, a Pokémon with two types that have a weakness and resistance to the same type receive neutral damage from that type, but the incorrect message is displayed.

#### Ghost identity unveiling

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Ghost sprite revealed on party menu return"

It is possible to reveal the identity of a [ghost](https://bulbapedia.bulbagarden.net/wiki/Ghost_\(Pok%C3%A9mon_Tower\) "Ghost (Pokémon Tower)") in [Pokémon Tower](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Tower "Pokémon Tower") without having a [Silph Scope](https://bulbapedia.bulbagarden.net/wiki/Silph_Scope "Silph Scope"). If the player views the [stats](https://bulbapedia.bulbagarden.net/wiki/Summary "Summary") of any Pokémon in the [party](https://bulbapedia.bulbagarden.net/wiki/Party "Party") and then returns to battle, then the ghost's identity will be revealed. However, this glitch is only graphical, and it is still impossible to [fight](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_battle "Pokémon battle") or catch it.

By [Wooggle](https://www.youtube.com/channel/UCgA3xOk7QY4MOYhc7EBFe0g)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=UMIowBT4Fck).**

#### Inverted sprites

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Inverted sprite (wSpriteFlipped persistence)"

*Main article: [Inverted sprite](https://bulbapedia.bulbagarden.net/wiki/Inverted_sprite "Inverted sprite")*

![](https://archives.bulbagarden.net/media/upload/3/33/Sprite_glitch.png)

Inverted sprites caused by a ♀.

Certain [glitch Pokémon](https://bulbapedia.bulbagarden.net/wiki/Glitch_Pok%C3%A9mon "Glitch Pokémon") can cause a bug to occur where all sprites in [battle](https://bulbapedia.bulbagarden.net/wiki/Battle "Battle") are mirrored and appear "broken". (With the exception of the opponent; the opponent appears flipped, but not broken, until it is hit by an attack.) It can be fixed by viewing the [Pokédex](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9dex "Pokédex") entry or [Summary](https://bulbapedia.bulbagarden.net/wiki/Summary "Summary") screen of a non-glitch [Pokémon](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon "Pokémon").

#### Link battle animation oversight

**Fixed in this ROM hack** (Minimize only) — see `VANILLA_BUGS.md`: "Link battle animation oversight (Minimize visual not applied when animations off)"

In link battles, some moves may not have consistent visual effects if one player has battle animations active but the other player does not.

For instance, only in this generation, [Acid Armor](https://bulbapedia.bulbagarden.net/wiki/Acid_Armor_\(move\) "Acid Armor (move)") turns the user invisible if the battle animations are active. Therefore, in a link battle, the same Pokémon can simultaneously appear visible in one game but invisible in the other game.

Similarly, [Minimize](https://bulbapedia.bulbagarden.net/wiki/Minimize_\(move\) "Minimize (move)") turns the user into a tiny generic sprite if the battle animations are active. Therefore, in a link battle, the same Pokémon can simultaneously appear as a tiny generic image in one game but as a regular Pokémon in the other game. This issue with Minimize was fixed in [Generation II](https://bulbapedia.bulbagarden.net/wiki/Generation_II "Generation II"), where this move turns the user into a tiny generic sprite regardless of the battle animations being active or inactive.

#### Mimic PP glitch

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Mimic PP glitch (max PP display shows copied move's PP)"

![](https://archives.bulbagarden.net/media/upload/6/69/Mimic_Horn_Drill_PP.png)

Horn Drill copied by Mimic with current PP higher than maximum PP (9/5 PP)

In this generation only, moves copied by [Mimic](https://bulbapedia.bulbagarden.net/wiki/Mimic_\(move\) "Mimic (move)") have an incorrect value displayed as their maximum PP in the list of moves. The maximum PP displayed is taken from the copied move, when in fact the maximum PP usable in battle is that of Mimic itself. The Pokémon's [summary](https://bulbapedia.bulbagarden.net/wiki/Summary "Summary") is unaffected by this glitch, and will display the correct maximum PP for Mimic.

Alternatively, if Mimic was called by [Mirror Move](https://bulbapedia.bulbagarden.net/wiki/Mirror_Move_\(move\) "Mirror Move (move)") or [Metronome](https://bulbapedia.bulbagarden.net/wiki/Metronome_\(move\) "Metronome (move)"), then the copied move's maximum PP is that of the move that called Mimic. If Mimic was acquired by [Transform](https://bulbapedia.bulbagarden.net/wiki/Transform_\(move\) "Transform (move)"), then the move copied by Mimic will use the PP that was given by Transform (instead of using the Pokémon's own PP).

If Mimic (as well as Mirror Move or Metronome) has its PP increased by any [PP Ups](https://bulbapedia.bulbagarden.net/wiki/PP_Up "PP Up"), this unused maximum PP displayed is affected by the PP Ups.

For instance, if Mimic copies [Tackle](https://bulbapedia.bulbagarden.net/wiki/Tackle_\(move\) "Tackle (move)") and currently has 9 PP, this can be displayed as " Tackle 9/35" (where "35" is Tackle's maximum PP with no PP Ups) or possibly " Tackle 9/56" (where "56" is Tackle's maximum PP with three PP Ups, which would be the value displayed if in fact the user's Mimic has three PP Ups). In cases like these, items such as [Ether](https://bulbapedia.bulbagarden.net/wiki/Ether "Ether") or [Elixir](https://bulbapedia.bulbagarden.net/wiki/Elixir "Elixir") can heal up to Mimic's true maximum PP, not up to the incorrect maximum PP displayed in battle.

Conversely, it is also possible to have more PP than the incorrect maximum value displayed. For example, if Mimic copies [Horn Drill](https://bulbapedia.bulbagarden.net/wiki/Horn_Drill_\(move\) "Horn Drill (move)") and currently has 9 PP, this can be displayed as " Horn Drill 9/5" (where "5" is Horn Drill's maximum PP, with no PP Ups)

This glitch was fixed in the [Pokémon Stadium series](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium_series "Pokémon Stadium series"), where the moves copied by Mimic have their maximum PP correctly displayed. In [Japanese Pokémon Stadium](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium_\(Japanese\) "Pokémon Stadium (Japanese)") and its sequel [Pokémon Stadium](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Stadium "Pokémon Stadium"), Mimic's current and maximum PP (alternatively, those of Mirror Move or Metronome if applicable) are displayed for the moves copied by Mimic.

#### Poison/Burn animation with 0 HP

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Poison/Burn animation with 0 HP"

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium and English Stadium? |
| --- | --- |

If a poisoned/burned Pokémon with low HP is confused and in the next turn loses its HP, the HP will be 0, but before it faints, the message and the animation of the poison/burn will appear, although the Pokémon doesn't have any HP. This also happen with a move which reduces the user's HP, like Take Down. This was fixed in Pokémon Gold/Silver.

By [LanceAndMissingNo.](https://www.youtube.com/channel/UCCheenv4-UJG9zDa_3kFBNw)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=sIp6YsFt1Jw).**

#### Substitute and Minimize glitch

If the enemy uses Substitute or Minimize and the player goes to view the stats of any Pokémon and then return to the battle, the sprites will be changed. The enemy will have the sprite of the Pokémon's player but broken, and the Pokémon of the player will have the Substitute or Minimize sprite. The sprite of the rival can change if the player goes to view the stats of any Pokémon in the team.

By [LanceAndMissingNo.](https://www.youtube.com/channel/UCCheenv4-UJG9zDa_3kFBNw)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=SOQjt7crzPY).**

#### Switching animation oversight

In link battles, the animation for switching a Pokémon is not consistent between the two players.

When a player switches a non-fainted Pokémon for another, the Pokémon who left is seen quickly shrinking as if returning to the Poké Ball. However, at the same time, from the point of view of the opponent, this Pokémon is seen quickly moving away horizontally instead of shrinking.

As an exception, this does not apply to the [Pikachu](https://bulbapedia.bulbagarden.net/wiki/Pikachu_\(Yellow\) "Pikachu (Yellow)") in Pokémon Yellow. It enters all battles by quickly moving horizontally into the screen, and leaves them by quickly moving away, as it is [not kept in a Poké Ball](https://bulbapedia.bulbagarden.net/wiki/Walking_Pok%C3%A9mon "Walking Pokémon"). Therefore, the Pikachu leaves battles using a consistent animation from the point of view of both players in a link battle.

That being said, the partner Pikachu has a different graphical quirk when entering link battles instead of leaving them: see [below](https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_I#Pikachu_entering_link_battles).

#### Substitute sprite vanishing

| ![](https://archives.bulbagarden.net/media/upload/thumb/a/a6/0050Diglett.png/50px-0050Diglett.png) | **This section is incomplete.**   Please feel free to edit this section to add missing information and complete it.   Reason: Is this present in Japanese Stadium? |
| --- | --- |

Using a sacrificial move like Explosion on a substitute and having the damage break the substitute prevents the user from fainting. The sprite of the user vanishes regardless.

By [ChickasaurusGL](https://www.youtube.com/channel/UCZz2ixp-5T6VeAPtAMQ5v5Q)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=lr05doU5oAQ).**

### Pokémon Red, Green, and Blue (Japanese)

#### Whirlwind text box overflow

In these games, if the player's Pokémon uses [Whirlwind](https://bulbapedia.bulbagarden.net/wiki/Whirlwind_\(move\) "Whirlwind (move)") on an enemy Pokémon with 5 characters in its name, the exclamation mark character overlaps with the border of the text box. This was corrected in the Japanese version of [Pokémon Yellow](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Yellow_Version "Pokémon Yellow Version") with the addition of a line break.

By [ChickasaurusGL](https://www.youtube.com/channel/UCZz2ixp-5T6VeAPtAMQ5v5Q)

**This video is not available on Bulbapedia; instead, you can watch the video on YouTube [here](https://www.youtube.com/watch?v=o-VeLoDMn9I).**

### Pokémon Yellow

#### Entering the first battle against the rival

In the first battle against the rival [Blue](https://bulbapedia.bulbagarden.net/wiki/Blue_\(game\) "Blue (game)"), the fact that [Pikachu](https://bulbapedia.bulbagarden.net/wiki/Pikachu_\(Yellow\) "Pikachu (Yellow)") was originally sent from a [Poké Ball](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9_Ball "Poké Ball") is ignored.

Usually, the partner Pikachu enters all battles in the game by sliding in horizontally, due to it being a [walking Pokémon](https://bulbapedia.bulbagarden.net/wiki/Walking_Pok%C3%A9mon "Walking Pokémon"), unlike other Pokémon who are sent from their Poké Balls. However, in the first battle, Pikachu enters the battle the same way despite the fact that it was sent from a Poké Ball at this point in the game.

#### Pikachu entering link battles

[Pikachu](https://bulbapedia.bulbagarden.net/wiki/Pikachu_\(Yellow\) "Pikachu (Yellow)") 's animation when entering a link battle is inconsistent between the two players.

The player's Pikachu enters the battle by quickly moving horizontally, which references the fact that it is [not kept in a Poké Ball](https://bulbapedia.bulbagarden.net/wiki/Walking_Pok%C3%A9mon "Walking Pokémon"). However, at the same time, from the point of view of the opponent, Pikachu is seen entering the battle from a Poké Ball instead.

This is a result of the opposing game treating the partner Pikachu as a regular Pokémon with no special animation for entering battles. This happens even if both players are playing Pokémon Yellow.

## References

<table><tbody><tr><th colspan="2"><div><a href="https://bulbapedia.bulbagarden.net/wiki/Glitch">Glitches</a> in the <a href="https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_games">Pokémon games</a></div></th></tr><tr><td colspan="2"><a href="https://bulbapedia.bulbagarden.net/wiki/List_of_Transform_glitches">Transform glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Glitch_Trainer">Glitch Trainers</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Cloning_glitches">Cloning glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Error_message">Error messages</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Arbitrary_code_execution">Arbitrary code execution</a></td></tr><tr><td colspan="2" height="1"></td></tr><tr><th><a href="https://bulbapedia.bulbagarden.net/wiki/Generation_I">Generation I</a></th><td><a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_I">Glitches</a> • <a>Battle glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_I">Overworld glitches</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/--_(move)">--</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/0_ERROR">0 ERROR</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Broken_hidden_items">Broken hidden items</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Cable_Club_escape_glitch">Cable Club escape glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Dual-type_damage_misinformation">Dual-type damage misinformation</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Experience#Experience_underflow_glitch">Experience underflow glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Fight_Safari_Zone_Pok%C3%A9mon_trick">Fight Safari Zone Pokémon trick</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Glitch_City">Glitch City</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Item_duplication_glitch">Item duplication glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Item_underflow">Item underflow</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Mew_glitch">Mew glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Old_man_glitch">Old man glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Pewter_Gym_skip_glitch">Pewter Gym skip glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_merge_glitch">Pokémon merge glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Rhydon_glitch">Rhydon glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Rival_twins_glitch">Rival twins glitch</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Select_glitches">Select glitches</a> (<a href="https://bulbapedia.bulbagarden.net/wiki/Dokokashira_door_glitch">dokokashira door glitch</a>, <a href="https://bulbapedia.bulbagarden.net/wiki/Second_type_glitch">second type glitch</a>) • <a href="https://bulbapedia.bulbagarden.net/wiki/Super_Glitch">Super Glitch</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Time_Capsule_exploit">Time Capsule exploit</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Walking_through_walls">Walking through walls</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/ZZAZZ_glitch">ZZAZZ glitch</a></td></tr><tr><td colspan="2" height="1"></td></tr><tr><th><a href="https://bulbapedia.bulbagarden.net/wiki/Generation_II">Generation II</a></th><td><a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_II">Glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_II">Battle glitches</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Bug-Catching_Contest_glitch">Bug-Catching Contest glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Celebi_Egg_glitch">Celebi Egg glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Coin_Case_glitches">Coin Case glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Experience#Experience_underflow_glitch">Experience underflow glitch</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Glitch_dimension">Glitch dimension</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Glitch_Egg">Glitch Egg</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Teru-sama">Teru-sama</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Time_Capsule_exploit">Time Capsule exploit</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Trainer_House_glitches">Trainer House glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/GS_Ball_mail_glitch">GS Ball mail glitch</a></td></tr><tr><td colspan="2" height="1"></td></tr><tr><th><a href="https://bulbapedia.bulbagarden.net/wiki/Generation_III">Generation III</a></th><td><a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_III">Glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_III">Battle glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_III">Overworld glitches</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Berry_glitch">Berry glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Dive_glitch">Dive glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Pomeg_glitch">Pomeg glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Glitzer_Popping">Glitzer Popping</a></td></tr><tr><td colspan="2" height="1"></td></tr><tr><th><a href="https://bulbapedia.bulbagarden.net/wiki/Generation_IV">Generation IV</a></th><td><a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_IV">Glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_IV">Battle glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_IV">Overworld glitches</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Acid_rain">Acid rain</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Global_Trade_System#Glitches_and_manipulation">GTS glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Pomeg_glitch">Pomeg glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Rage_glitch">Rage glitch</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Surf_glitch">Surf glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Tweaking">Tweaking</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Pal_Park_Retire_glitch">Pal Park Retire glitch</a></td></tr><tr><td colspan="2" height="1"></td></tr><tr><th><a href="https://bulbapedia.bulbagarden.net/wiki/Generation_V">Generation V</a></th><td><a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_V">Glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_V">Battle glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_V">Overworld glitches</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Charge_Beam_additional_effect_chance_glitch">Charge Beam additional effect chance glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Charge_move_replacement_glitch">Charge move replacement glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Choice_item_lock_glitch">Choice item lock glitch</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Frozen_Zoroark_glitch">Frozen Zoroark glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Sky_Drop_glitch">Sky Drop glitch</a></td></tr><tr><td colspan="2" height="1"></td></tr><tr><th><a href="https://bulbapedia.bulbagarden.net/wiki/Generation_VI">Generation VI</a></th><td><a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_VI">Glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_VI">Battle glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_VI">Overworld glitches</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Charge_Beam_additional_effect_chance_glitch">Charge Beam additional effect chance glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Charge_move_replacement_glitch">Charge move replacement glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Choice_item_lock_glitch">Choice item lock glitch</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Lumiose_City_save_glitch">Lumiose City save glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Symbiosis_Eject_Button_glitch">Symbiosis Eject Button glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Toxic_sure-hit_glitch">Toxic sure-hit glitch</a></td></tr><tr><td colspan="2" height="1"></td></tr><tr><th><a href="https://bulbapedia.bulbagarden.net/wiki/Generation_VII">Generation VII</a></th><td><a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_VII">Glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_battle_glitches_in_Generation_VII">Battle glitches</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_overworld_glitches_in_Generation_VII">Overworld glitches</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Charge_Beam_additional_effect_chance_glitch">Charge Beam additional effect chance glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Charge_move_replacement_glitch">Charge move replacement glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Choice_item_lock_glitch">Choice item lock glitch</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Toxic_sure-hit_glitch">Toxic sure-hit glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Rollout_storage_glitch">Rollout storage glitch</a></td></tr><tr><td colspan="2" height="1"></td></tr><tr><th><a href="https://bulbapedia.bulbagarden.net/wiki/Generation_VIII">Generation VIII</a></th><td><a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_VIII">Glitches</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Charge_Beam_additional_effect_chance_glitch">Charge Beam additional effect chance glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Charge_move_replacement_glitch">Charge move replacement glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Choice_item_lock_glitch">Choice item lock glitch</a><br><a href="https://bulbapedia.bulbagarden.net/wiki/Toxic_sure-hit_glitch">Toxic sure-hit glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Rollout_storage_glitch">Rollout storage glitch</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Party_item_offset_glitch">Party item offset glitch</a></td></tr><tr><td colspan="2" height="1"></td></tr><tr><th><a href="https://bulbapedia.bulbagarden.net/wiki/Generation_IX">Generation IX</a></th><td><a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Generation_IX">Glitches</a></td></tr><tr><td colspan="2" height="1"></td></tr><tr><th><a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitch_effects">Glitch effects</a></th><td><a href="https://bulbapedia.bulbagarden.net/wiki/Game_freeze">Game freeze</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Glitch_battle">Glitch battle</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Glitch_song">Glitch song</a><br>Gen I only: <a href="https://bulbapedia.bulbagarden.net/wiki/Glitch_screen">Glitch screen</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/TMTRAINER_effect">TMTRAINER effect</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/Inverted_sprite">Inverted sprite</a><br>Gen II only: <a href="https://bulbapedia.bulbagarden.net/wiki/Glitch_dimension">Glitch dimension</a></td></tr><tr><td colspan="2" height="1"></td></tr><tr><th><a href="https://bulbapedia.bulbagarden.net/wiki/Bulbapedia:Lists">Lists</a></th><td><a href="https://bulbapedia.bulbagarden.net/wiki/Glitch">Glitches</a> (<a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Pok%C3%A9mon_GO">GO</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_Pok%C3%A9mon_HOME">HOME</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_the_Pok%C3%A9mon_Mystery_Dungeon_series">Mystery Dungeon</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_(TCG_GB)">TCG GB</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_in_spin-off_games">Spin-off</a>)<br><a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitch_Pok%C3%A9mon">Glitch Pokémon</a> (<a href="https://bulbapedia.bulbagarden.net/wiki/List_of_Pok%C3%A9mon_by_index_number_in_Generation_I">Gen I</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_Pok%C3%A9mon_by_index_number_in_Generation_II">Gen II</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_Pok%C3%A9mon_by_index_number_in_Generation_III">Gen III</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_Pok%C3%A9mon_by_index_number_in_Generation_IV">Gen IV</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_Pok%C3%A9mon_by_index_number_in_Generation_V">Gen V</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_Pok%C3%A9mon_by_index_number_in_Generation_VI">Gen VI</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_Pok%C3%A9mon_by_index_number_in_Generation_VII">Gen VII</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_Pok%C3%A9mon_by_index_number_in_Generation_VIII">Gen VIII</a>)<br><a href="https://bulbapedia.bulbagarden.net/wiki/Glitch_move">Glitch moves</a> (<a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitch_moves_in_Generation_I">Gen I</a>) • <a href="https://bulbapedia.bulbagarden.net/wiki/Glitch_type">Glitch types</a> (<a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitch_types_in_Generation_I">Gen I</a> • <a href="https://bulbapedia.bulbagarden.net/wiki/List_of_glitch_types_in_Generation_II">Gen II</a>)</td></tr></tbody></table>

  

| ![](https://archives.bulbagarden.net/media/upload/thumb/9/97/Project_GlitchDex_logo.png/80px-Project_GlitchDex_logo.png) | This glitch Pokémon article is part of **[Project GlitchDex](https://bulbapedia.bulbagarden.net/wiki/Bulbapedia:Project_GlitchDex "Bulbapedia:Project GlitchDex")**, a [Bulbapedia project](https://bulbapedia.bulbagarden.net/wiki/Bulbapedia:Projects "Bulbapedia:Projects") that aims to write comprehensive articles on glitches in the Pokémon games. |
| --- | --- |