---
title: "List of Transform glitches"
author: "Bulbapedia"
published: 2026-03-12T07:15:33Z
source: "https://bulbapedia.bulbagarden.net/wiki/List_of_Transform_glitches"
domain: "bulbapedia.bulbagarden.net"
language: "en"
word_count: 4542
---

These are glitches involving the move [Transform](https://bulbapedia.bulbagarden.net/wiki/Transform_(move)).

## Generation I

### -- (glitch move from SELECT swap)

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Transform move swap protection" (on `dashed/transform-swap`)

In Generation I, if a Pokémon transforms into another Pokémon that knows more moves and, while transformed, switches around the copied moves with the SELECT button, after the battle the Pokémon will not have Transform and will instead have the glitch move --. This can also be done with Pokémon that have used Mimic in order to use Transform.

### Ditto glitch

**Fixed in this ROM hack** — the Mew glitch / Trainer Fly is fixed on `dashed/overworld-fixes` (BIT_SEEN_BY_TRAINER cleared on map entry).

The primary method of the Mew glitch involves having a wild Ditto transform into the player's Pokémon, as this copies the Special stat, which is used to modify the species of Pokémon encountered.

### Color loss glitch

**Not applicable** — this only affects Pokémon Red and Blue. Per the original article: "this does not happen in Pokémon Yellow."

In Pokémon Red and Blue (when played in color), a transformed Pokémon initially keeps its own color palette when transformed, but it changes into the Ditto color palette once its sprite is reloaded.

### Level up moveset glitch

**Not fixed** — complex engine behavior; the battle system is not designed for level-ups during Transform.

Only in Generation I, if a transformed Pokémon levels up and learns a new move, it will use its original moveset (unaffected by Transform) from this point onwards in the current battle. Additionally, any PP lost from this point onwards in the current battle will not affect the Pokémon's actual PP as shown in the summary.

### Transform + Mirror Move/Metronome PP error

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Transform + Mirror Move/Metronome PP error"

If Mirror Move or Metronome is used by a transformed Pokémon, the move of the same slot in that Pokémon's actual moveset will have its PP increased by 1. This occurs even if there is no move in that slot, which can prevent the use of Struggle or cause a softlock when targeted by Disable. PP of slots without moves will not be reset to 0 when healing at a Pokémon Center; the glitch can only be repaired by repeating it 256 times, at which point the PP count will overflow to 0.

### Unused catch rate

**Not fixed** — no gameplay effect. Transform copies an unused catch rate value from the target Pokémon, but the catch rate from the original species is always used when catching.

## Generations I and II

### Transform assumption oversight

**Fixed in this ROM hack** — see `VANILLA_BUGS.md`: "Transformed Pokemon assumed to be Ditto when catching" (on `dashed/battle-bugs`)

Only in Generations I and II, if the player catches any transformed Pokémon, that Pokémon is permanently converted to a Ditto. All the moves previously known by this Pokémon are lost, as its only move when caught will be Transform.

### Transform DV manipulation glitch

**Not fixed** — DV copying is an inherent part of Transform's mechanics. Arguably not a bug.

If the opposing Pokémon uses Transform twice in the same battle, it will retain the DVs of the penultimate Pokémon it transformed into.

### Shiny Ditto glitch

**Not applicable** — Gen I has no Shiny system. Only relevant when traded to Gen II.

In Generations I and II, if a non-Shiny wild Ditto uses Transform on a Shiny Pokémon owned by the player, and then uses Transform again in the same battle, this Ditto will become permanently Shiny.

### Transform gender change glitch

**Not applicable** — Gen I has no gender system. Only relevant in Gen II.

This glitch can be used to change the gender of a male or female Pokémon in battle based on the Transform DV manipulation glitch.

## Generation II and later

The remaining glitches (Roar/Whirlwind shared DV, Sketch glitch, Shiny Transform switch, Mirror Move failure, Wild Ditto's Transform PP, Gen III/IV/IX glitches) are not applicable to this Pokémon Yellow ROM hack.
