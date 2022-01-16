# Pokémon Yellow [![Build Status][travis-badge]][travis]

This is a disassembly of Pokémon Yellow.

It builds the following ROMs:

- Pokemon Yellow (UE) [C][!].gbc `sha1: cc7d03262ebfaf2f06772c1a480c7d9d5f4a38e1`
- YELLMONS.GB (debug build) `sha1: d44e96eddfbdad633cbe4e6e64915e9e198974b0`

To set up the repository, see [**INSTALL.md**](INSTALL.md).

## Dashed's patch

- Forked from https://github.com/dannye/pokeyellow-gen-II with gen 2 graphics.

#### Quality of life improvements\*

- Remove artificial save delay
- Fix 1/256 miss glitch - https://bulbapedia.bulbagarden.net/wiki/List_of_glitches_(Generation_I)#1.2F256_miss_glitch
- Add warp text speed mode.

#### Addition of missing Pokemon

Reference https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Yellow_Version#Missing_Pok.C3.A9mon

1. Weedle - Route 2, Viridian Forest
2. Kakuna - Route 2, Viridian Forest
3. Ekans - Routes 4
4. Raichu - Power Plant
5. Meowth - Route 5
6. Koffing - Pokemon Mansion Floor 1 and 2
7. Weezing - Pokemon Mansion Floor 2
8. Jynx - Bottom Level of Seafoam Islands (walking)
9. Electabuzz - Power Plant
10. Magmar - Pokemon Mansion Floor 3 and Basment Floor One

#### Addition of Pokemon

- Eevee - Route 16. There was only one Eevee in the game at the Pokémon Mansion in Celadon City.
- Hitmonlee - Victory Road (floor with Moltres)
- Hitmonchan - Victory Road (floor with Moltres)
- Omanyte - Bottom floor of Seafoam Islands (water)
- Kabuto - Bottom floor of Seafoam Islands (water)
- Mew - Wild in the last floor of Unknown Dungeon

#### Fix Impossible Evolutions

- Kadabra evolves into Alakazam at Level 36
- Graveler evolves into Golem at Level 36

#### Misc additions

- Add the "PRESENTS" copy under the Game Freak logo in the intro animation.
  - Ref: https://github.com/pret/pokered/wiki/Restore-the-PRESENTS-subtitle-under-the-Game-Freak-logo-in-the-intro-animation.

## Build notes

```
make all
make yellow
make yellow_debug
```

## References for Assembly

- https://rgbds.gbdev.io/docs/v0.5.1/gbz80.7
- http://z80-heaven.wikidot.com/

## See also

- **Discord:** [pret][discord]
- **IRC:** [freenode#pret][irc]

Other disassembly projects:

- [**Pokémon Red/Blue**][pokered]
- [**Pokémon Gold/Silver**][pokegold]
- [**Pokémon Crystal**][pokecrystal]
- [**Pokémon Pinball**][pokepinball]
- [**Pokémon TCG**][poketcg]
- [**Pokémon Ruby**][pokeruby]
- [**Pokémon FireRed**][pokefirered]
- [**Pokémon Emerald**][pokeemerald]

[pokered]: https://github.com/pret/pokered
[pokegold]: https://github.com/pret/pokegold
[pokecrystal]: https://github.com/pret/pokecrystal
[pokepinball]: https://github.com/pret/pokepinball
[poketcg]: https://github.com/pret/poketcg
[pokeruby]: https://github.com/pret/pokeruby
[pokefirered]: https://github.com/pret/pokefirered
[pokeemerald]: https://github.com/pret/pokeemerald
[discord]: https://discord.gg/d5dubZ3
[irc]: https://kiwiirc.com/client/irc.freenode.net/?#pret
[travis]: https://travis-ci.org/pret/pokeyellow
[travis-badge]: https://travis-ci.org/pret/pokeyellow.svg?branch=master
