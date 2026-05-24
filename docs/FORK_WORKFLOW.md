# Custom Pokemon Yellow Fork Workflow

This document describes the branch structure and workflow for maintaining a custom Pokemon Yellow fork with independent feature branches that can be selectively combined.

## Overview

This fork uses a **modular feature branch strategy** where:

- `master` tracks upstream (dannye/pokeyellow-gen-2-gfx)
- `dashed/tests` provides a shared test harness foundation based on `master`
- Each feature group lives in its own branch based on `dashed/tests`, with its own test file(s)
- `dashed/docs` holds fork-level documentation in `docs/` (this file, VANILLA_BUGS.md, etc.)
- `dashed-patch` is a merge commit that combines all desired features + docs
- Jujutsu (jj) is used for version control alongside Git

This approach allows:

- Easy updates when upstream releases new changes
- Selective feature inclusion (enable/disable features by changing the merge)
- Clean separation of concerns
- Feature-specific tests co-located with the code they verify
- Simple conflict resolution per-feature

### Why docs live in their own branch

Documentation files like FORK_WORKFLOW.md and VANILLA_BUGS.md describe the fork itself, not any single feature. Previously, these lived as linear commits **on top of** the `dashed-patch` merge commit. This caused them to be silently lost whenever `dashed-patch` was recreated via `jj new branch1 branch2 ...` — the merge only includes files from its parent branches, so the old linear doc commits were orphaned.

Putting docs in `dashed/docs` (a proper bookmarked branch and merge parent) ensures they survive merge recreation.

## Remotes

| Remote | URL | Purpose |
|--------|-----|---------|
| `origin` | `git@github.com:dashed/pokeyellow-gen-II.git` | Fork (push target) |
| `author` | `git@github.com:dannye/pokeyellow-gen-2-gfx.git` | Upstream (fetch-only) |

**Note:** SSH push may fail on some setups. Use HTTPS as fallback:
```bash
git push https://github.com/dashed/pokeyellow-gen-II.git branch_name
```

## Branch Structure

```
master (upstream @ author/master)
│
└── dashed/tests (15 commits — shared test harness foundation)
    │
    │── Bug fix branches:
    │
    ├── dashed/accuracy-crit (3 commits, 3 fixes)
    │   ├── fix: 1/256 miss, 1/256 crit, Focus Energy quartering→quadrupling
    │   └── tests: accuracy, crit, link_regression
    │
    ├── dashed/battle-bugs (46 commits, 44 fixes)
    │   ├── fix: Substitute, dual-type effectiveness, Drain/Eater,
    │   │   Counter/Bide, Psywave, Fly/Dig invulnerability, healing
    │   │   255/511, Haze freeze/recharge, Exp. All, CooltrainerF AI,
    │   │   stat mods + badge boosts, Toxic/Leech Seed, Red bar alarm,
    │   │   Mirror Move desync, Mimic level-up, learnset skip,
    │   │   Jump Kick crash, Hyper Beam+Sleep, exp underflow,
    │   │   division by zero, defrost forcing, Trapping+Sleep,
    │   │   Substitute+Confusion, switch-out msg, AI trainer HUD,
    │   │   Pikachu voice cry, Minimize anim, victory music, and more
    │   └── tests: 40+ test files (substitute, effectiveness, bide, etc.)
    │
    ├── dashed/ghost-battle (1 commit, 2 fixes)
    │   ├── fix: Ghost Pokédex seen flag, ghost sprite reload on party menu
    │   └── test: ghost_pokedex
    │
    ├── dashed/item-fixes (10 commits, 9 fixes)
    │   ├── fix: PP restore PP Ups, Transform/Ditto catch rate RNG,
    │   │   status cure stat reapply, Poké Doll ghost Marowak,
    │   │   Item Finder coordinate 0, vending machine price,
    │   │   Repel override, catch rate rejection sampling,
    │   │   friendship item happiness
    │   └── tests: pp_restore, transform_catch, status_cure, pokedoll,
    │       itemfinder, struggle, vending_machine, repel_override,
    │       friendship_item, catch_rate
    │
    ├── dashed/overworld-fixes (48 commits, 42 fixes)
    │   ├── fix: NPC movement/placement, Route 16 sign, invisible tree,
    │   │   ledge-NPC, bicycle hole, escape sprite, binoculars, trainer
    │   │   text, elevator same-floor, Pikachu off-screen, Walking Through
    │   │   Walls, Save Surf exploit, Repel step count + save, Pokédex
    │   │   assumption, healthy party deposit, Safari Gate nugget,
    │   │   Pallet NPC, Oak's Poké Balls, Cycling Road bypass, hidden
    │   │   coins, save dialog, MtMoon battles, PokemonTower2F coords,
    │   │   Fuji warp, slot machine fixes, dungeon map transitions,
    │   │   boulder smoke OAM, OAM DMA tearing, tile pair collision,
    │   │   healing machine tiles, splash stars, Pewter youngster,
    │   │   Oak's lab music/stuck, fossil cry, hidden item jingle,
    │   │   new game flags, ED tile loading, and more
    │   └── tests: 45+ test files + ROM0 tail-call optimizations (−22 bytes)
    │
    ├── dashed/glitch-safety (9 commits, 9 fixes)
    │   ├── fix: MissingNo. SRAM corruption + sprite clamping,
    │   │   ZZAZZ trainer data, Super Glitch move names,
    │   │   Yami Shop name buffer, item duplication Pokédex flags,
    │   │   inverted sprite bit, Pokémon merge removal,
    │   │   Rare Candy level cap, glitch move PP bounds
    │   └── tests: missingno, zzazz, super_glitch, yami_shop,
    │       item_duplication, inverted_sprite, pokemon_merge,
    │       rare_candy_level_cap, glitch_moves
    │
    │── Feature branches:
    │
    ├── dashed/qol (2 commits)
    │   ├── feat: WARP text speed (instant, no per-character delay)
    │   ├── fix: remove artificial save delay
    │   └── tests: save, warp_text, visual
    │
    ├── dashed/cosmetic (10 commits, 1 feat + 6 fixes)
    │   ├── feat: restore 'PRESENTS' text in Game Freak intro
    │   ├── fix: wavy screen top 3 lines, slide animation tearing,
    │   │   Double Edge opponent animation, Trainer Card DMG delay,
    │   │   pitch slide high-byte borrow, Route 8 truncated text
    │   └── tests: cosmetic (golden image), trainer_card_dmg,
    │       slide_tearing, double_edge_anim, pitch_slide, wavy_screen
    │
    ├── dashed/evolutions (5 commits)
    │   ├── feat: trade evolutions → level 36 (Kadabra, Graveler,
    │   │   Machoke, Haunter)
    │   └── test: evolutions (verify level 36 ROM bytes)
    │
    ├── dashed/wild-pokemon (18 commits)
    │   ├── feat: version-exclusive wild encounters (Weedle, Kakuna,
    │   │   Ekans, Raichu, Meowth, Koffing, Weezing, Jynx,
    │   │   Electabuzz, Magmar, Eevee, Hitmonlee, Hitmonchan,
    │   │   Omanyte, Kabuto, Mew)
    │   └── tests: wild_pokemon (ROM bytes), mew_encounter (emulator)
    │
    ├── dashed/pikachu-surf (2 commits)
    │   ├── feat: let Pikachu learn HM Surf
    │   └── test: pikachu_surf (verify HM Surf learnset ROM bytes)
    │
    ├── dashed/surge-trash (1 commit, 1 fix)
    │   └── fix: Lt. Surge gym trash can second lock randomization
    │
    ├── dashed/transform-swap (1 commit, 1 fix)
    │   └── fix: allow move swap while transformed (protect party data)
    │
    ├── dashed/item-overflow (1 commit, 1 fix)
    │   └── fix: prevent item stack overflow past inventory buffer
    │
    │── Documentation:
    │
    ├── dashed/docs (based on dashed/tests)
    │   └── docs/
    │       ├── FORK_WORKFLOW.md (this file)
    │       ├── VANILLA_BUGS.md (bug documentation)
    │       ├── REMAINING_GLITCHES.md (unfixed glitches audit)
    │       ├── TEST_COVERAGE_ANALYSIS.md (test gap analysis)
    │       └── (Bulbapedia reference copies)
    │
    └── dashed-patch (15-parent merge = integration branch)
```

### Branch Descriptions

| Branch | Base | Commits | Fixes | Purpose | Key Files |
|--------|------|---------|-------|---------|-----------|
| `dashed/tests` | `master` | 15 | — | Shared test harness (Rust/boytacean) | `tests/` |
| `dashed/accuracy-crit` | `dashed/tests` | 3 | 3 | 1/256 miss, 1/256 crit, Focus Energy | `engine/battle/core.asm` |
| `dashed/battle-bugs` | `dashed/tests` | 46 | 44 | Battle state, damage, animation bugs | `engine/battle/core.asm`, `engine/battle/effects.asm`, `engine/battle/move_effects/`, `engine/battle/trainer_ai.asm` |
| `dashed/ghost-battle` | `dashed/tests` | 1 | 2 | Ghost battle visual fixes | `engine/battle/core.asm` |
| `dashed/item-fixes` | `dashed/tests` | 10 | 9 | Item usage bugs | `engine/items/item_effects.asm`, `engine/items/itemfinder.asm`, `engine/events/vending_machine.asm` |
| `dashed/overworld-fixes` | `dashed/tests` | 48 | 42 | Overworld, NPC, map, slot machine bugs | `engine/overworld/`, `home/`, `engine/slots/`, `scripts/`, `data/maps/`, `data/events/` |
| `dashed/glitch-safety` | `dashed/tests` | 9 | 9 | Glitch Pokémon safeguards | `engine/battle/`, `engine/pokemon/`, `home/uncompress.asm`, `home/names2.asm` |
| `dashed/qol` | `dashed/tests` | 2 | — | WARP text speed, save delay removal | `engine/menus/options.asm`, `home/print_text.asm` |
| `dashed/cosmetic` | `dashed/tests` | 10 | 6 | PRESENTS subtitle, animation/audio fixes | `engine/movie/intro.asm`, `engine/battle/animations.asm`, `audio/engine_1.asm` |
| `dashed/evolutions` | `dashed/tests` | 5 | — | Trade evolutions → level 36 | `data/pokemon/evos_moves.asm` |
| `dashed/wild-pokemon` | `dashed/tests` | 18 | — | Version-exclusive wild encounters | `data/wild/maps/` |
| `dashed/pikachu-surf` | `dashed/tests` | 2 | — | Pikachu learns HM Surf | `data/pokemon/base_stats/pikachu.asm` |
| `dashed/surge-trash` | `dashed/tests` | 1 | 1 | Lt. Surge trash can randomization | `engine/events/hidden_events/` |
| `dashed/transform-swap` | `dashed/tests` | 1 | 1 | Move swap while transformed | `engine/battle/core.asm` |
| `dashed/item-overflow` | `dashed/tests` | 1 | 1 | Item stack overflow prevention | `engine/items/inventory.asm` |
| `dashed/docs` | `dashed/tests` | — | — | Fork documentation | `docs/FORK_WORKFLOW.md`, `docs/VANILLA_BUGS.md` |
| `dashed-patch` | (15-parent merge) | — | ~118 | Integration branch | (merge only) |

Branches that share `engine/battle/core.asm` (accuracy-crit, battle-bugs, ghost-battle, transform-swap) modify different functions within the file, so jj/git auto-merges them cleanly.

See `docs/VANILLA_BUGS.md` for a complete list of all fixed bugs with code-level details, references, and links to [Glitch City Wiki](https://glitchcity.wiki/wiki/) and [Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/).

## Jujutsu (jj) Setup

This repo uses jj in colocated mode, meaning both `jj` and `git` commands work. The `.jj` directory is already initialized.

### Why jj?

- **Automatic rebasing**: When you rebase a parent, descendants auto-rebase
- **First-class conflicts**: Conflicts are stored in commits, resolve when convenient
- **Operation log**: Every operation can be undone with `jj undo`
- **Change IDs**: Stable identifiers that survive rebases (unlike git commit hashes)
- **Multi-parent commits**: Native support for merge commits with 3+ parents

### Key jj Concepts

```bash
# Bookmarks = Git branches
jj bookmark list                    # List all bookmarks

# Working copy IS a commit
jj status                           # See current state
jj diff                             # See changes in working copy

# Change IDs vs Commit IDs
# - Change ID (e.g., wlvtootm): stable across rewrites
# - Commit ID (e.g., 7f026681): changes when commit is modified
```

## Updating from Upstream

When upstream (dannye/pokeyellow-gen-2-gfx) has new changes:

### Step 1: Fetch upstream changes

```bash
jj git fetch --remote author
```

### Step 2: Update master bookmark

```bash
jj bookmark set master -r author/master --allow-backwards
```

### Step 3: Rebase test foundation onto new master

```bash
# Rebase dashed/tests onto new master
# jj automatically rebases all descendants (feature branches + integration merge)
jj rebase -b dashed/tests -d master
```

This finds all root commits of the feature branches (commits that are ancestors of `dashed-patch` but not ancestors of `master`) and rebases them onto the updated `master`. Since jj auto-rebases descendants, all feature branch tips and the integration merge are updated automatically.

### Step 4: Resolve any conflicts

```bash
# Check for conflicts
jj log -r 'conflicts()'

# For each conflicted commit:
jj edit <conflicted-change-id>     # Edit the conflicted commit directly
# Resolve conflicts in files
jj new                              # Move working copy forward

# Alternative: work on top of conflict, then squash
jj new <conflicted-change-id>
# Edit files to resolve
jj squash                           # Fold resolution into parent
```

### Step 5: Push updated branches

```bash
# Preview what will be pushed
jj git push --dry-run --tracked

# Push
jj git push --tracked
```

**Note:** Since rebasing rewrites commit hashes, you'll need to force-push. jj handles this automatically when pushing tracked bookmarks.

## Adding a New Feature

### Step 1: Create feature branch from dashed/tests

```bash
jj new dashed/tests -m "feat: description of change"
jj bookmark create dashed/new-feature
```

### Step 2: Develop the feature

```bash
# Make changes — they're auto-tracked
# When done with a logical unit:
jj new -m "next part of feature"

# Or edit the description:
jj describe -m "better description"
```

### Step 3: Add to integration branch

Recreate the `dashed-patch` integration merge with the new feature included as an additional parent:

```bash
jj new dashed/accuracy-crit dashed/battle-bugs dashed/ghost-battle \
  dashed/item-fixes dashed/overworld-fixes dashed/glitch-safety \
  dashed/qol dashed/cosmetic dashed/evolutions dashed/wild-pokemon \
  dashed/pikachu-surf dashed/surge-trash dashed/transform-swap \
  dashed/item-overflow dashed/docs dashed/new-feature \
  -m "integration: 15-parent merge of all feature branches"
jj bookmark set dashed-patch -r @ --allow-backwards
```

### Editing an existing feature

```bash
# Edit a commit in-place
jj edit <change-id>

# Make changes — they modify that commit directly
# All descendants (including the integration merge) are automatically rebased

# Return to working on the tip
jj new dashed-patch
```

## Removing a Feature

To remove a feature, simply omit that branch from the parents when recreating the integration merge:

```bash
# Example: remove dashed/cosmetic from the integration
jj new dashed/accuracy-crit dashed/battle-bugs dashed/ghost-battle \
  dashed/item-fixes dashed/overworld-fixes dashed/glitch-safety \
  dashed/qol dashed/evolutions dashed/wild-pokemon dashed/pikachu-surf \
  dashed/surge-trash dashed/transform-swap dashed/item-overflow \
  dashed/docs \
  -m "integration: merge all feature branches"
jj bookmark set dashed-patch -r @ --allow-backwards
```

**Important:** Always include `dashed/docs` — omitting it will silently drop all fork documentation from the merge.

## The Integration Branch (dashed-patch)

`dashed-patch` is a **merge commit with 15 parents**. It combines all feature branches plus docs into a single working build.

### How it works

```bash
# Create a merge with multiple parents:
jj new <branch1> <branch2> <branch3> ... -m "integration message"
```

When any parent branch is updated (e.g., after rebasing onto new upstream), the merge commit is automatically recreated by jj.

### Avoiding file loss during merge recreation

Files get lost from `dashed-patch` in two ways:

1. **Uncommitted doc files on top of the merge.** If you add files as linear commits on top of the merge (not in a bookmarked branch), they are orphaned when `jj new` recreates the merge. **Fix:** Put all non-feature files in `dashed/docs` or another dedicated branch included as a merge parent.

2. **Stale feature branch bases.** If `dashed/tests` gets new commits after feature branches diverge, the new files won't appear in the merge (since they're not in any parent branch's tree). **Fix:** Periodically rebase all feature branches onto the latest `dashed/tests`:
   ```bash
   jj rebase -b dashed/accuracy-crit -d dashed/tests
   # Repeat for each branch, or use:
   jj rebase -b dashed-patch -d dashed/tests
   ```

## Building

```bash
# Build all targets (pokeyellow.gbc + pokeyellow_debug.gbc)
make -j$(nproc)

# Clean build artifacts
make clean
```

Output ROMs:

| File | Description |
|------|-------------|
| `pokeyellow.gbc` | Main ROM |
| `pokeyellow_debug.gbc` | Debug build |

### Running Tests

The test harness uses Rust with the boytacean Game Boy emulator. All test targets depend on `pokeyellow.gbc` (built automatically if needed).

```bash
make test               # Run all tests (RUST_TEST_THREADS=4)
make test-quick         # Skip exhaustive accuracy test (faster iteration)
make test-exhaustive    # Run only the exhaustive accuracy test
make clippy             # Run Rust linter
make fmt-check          # Check Rust formatting
make rust-check         # Run clippy + fmt-check + test (full CI check)
```

`RUST_TEST_THREADS` defaults to 4 to limit RAM usage from parallel GameBoy emulator instances. Override with `make test RUST_TEST_THREADS=2`.

**Test stats:** 145 test files, 1,135 test functions across all branches.

### Local Build Verification with Docker

The CI uses RGBDS v1.0.1 (on Ubuntu and macOS). If your local RGBDS version differs, builds may behave differently — especially around ROM bank size limits. Use Docker to verify with the exact CI toolchain:

```bash
make clean
docker run --rm -v "$(pwd):/repo" -w /repo ubuntu:latest bash -c '
  apt-get update -qq && \
  apt-get install -yqq git make gcc g++ libpng-dev bison pkg-config > /dev/null 2>&1 && \
  cd /tmp && git clone --depth 1 --branch v1.0.1 https://github.com/gbdev/rgbds.git 2>/dev/null && \
  cd rgbds && sudo make -j$(nproc) install > /dev/null 2>&1 && \
  cd /repo && rgbasm --version && make -j$(nproc)
'
```

### ROM0 (Home Section) Space Constraints

The Home section (`home/*.asm`) is placed in ROM0 (`$0150`–`$3FFF`), which has a hard size limit. Code added to any `home/` file counts toward this budget. ROM0 is nearly full in the upstream project, so any additions to `home/` files must be carefully sized.

Two rounds of `call BankswitchCommon / ret` → `jp BankswitchCommon` tail-call optimization on `dashed/overworld-fixes` freed 22 bytes (8 + 14) in ROM0, offsetting the OAM DMA fix and other feature additions.

If the linker reports `overflow ROM0 by N bytes`, you need to either:
- **Reduce code size** in `home/` files (optimize instructions, remove features)
- **Move logic** out of `home/` into a banked section (requires `farcall` to invoke)
- **Apply tail-call optimization**: Find remaining `call X / ret` patterns and convert to `jp X` (−1 byte each)

Note: different RGBDS versions may produce slightly different code sizes for the same source, so always verify with the CI version (v1.0.1).

### Compile-Time Assertions

This fork uses RGBDS `ASSERT` directives as compile-time tests. These fail the build if invariants are violated, cost zero bytes in the final ROM, and catch bugs at assembly time rather than at runtime.

#### Constant ordering assertions (`constants/ram_constants.asm`)

```asm
ASSERT TEXT_DELAY_WARP == 0, "TEXT_DELAY_WARP must be 0 (PrintLetterDelay uses jr z)"
ASSERT TEXT_DELAY_WARP < TEXT_DELAY_FAST
ASSERT TEXT_DELAY_FAST < TEXT_DELAY_MEDIUM
ASSERT TEXT_DELAY_MEDIUM < TEXT_DELAY_SLOW
```

These guard the text speed constants used by `PrintLetterDelay` in `home/print_text.asm`. The WARP == 0 assertion is critical: `PrintLetterDelay` uses `jr z` (jump if zero) to skip per-character delay entirely for WARP mode. If WARP were ever changed to a non-zero value, the WARP feature would silently break.

The ordering assertions ensure the delay values increase monotonically, which the options menu cycling logic depends on.

#### Table length assertions (`engine/menus/options.asm`)

```asm
.Strings:
; entries correspond to OPT_TEXT_SPEED_* constants
    table_width 2
    dw .Fast
    dw .Mid
    dw .Slow
    dw .Warp
    assert_table_length NUM_TEXT_SPEED_OPTS
```

`table_width` and `assert_table_length` are upstream macros (from `macros/asserts.asm`) that verify a data table has exactly the expected number of entries. If someone adds a new text speed constant but forgets to add its string pointer (or vice versa), the build fails.

#### Adding new assertions

For constant invariants:
```asm
ASSERT <condition>, "Error message explaining why this must hold"
```

For data tables, wrap with `table_width <bytes_per_entry>` before and `assert_table_length <expected_count>` after:
```asm
table_width 2          ; each entry is 2 bytes (dw = pointer)
dw .Entry1
dw .Entry2
assert_table_length 2  ; must match the number of entries
```

### Accuracy Check: Optimal Rounding to N/255

The 1/256 miss bug fix in `engine/battle/core.asm` uses optimal rounding to best approximate the intended hit probability.

**The problem:** Move accuracy is stored as a byte N = floor(P × 255 / 100), where P is the intended percentage (e.g. 95% → N=242). The ideal hit probability is N/255, but `BattleRandom` produces 256 values (0–255), so we must round:

| Strategy | Hit probability | Error from N/255 |
|----------|----------------|-------------------|
| `random < N` | N/256 | undershoots by N/(256×255) |
| `random ≤ N` | (N+1)/256 | overshoots by (255−N)/(256×255) |

The errors are equal when N = 127.5. So:
- **N ≥ 128** (bit 7 set): `≤` is more accurate
- **N < 128** (bit 7 clear): `<` is more accurate

Our implementation checks `bit 7, b` when `random == accuracy` to select the optimal rounding for each accuracy value. This gives the closest possible approximation to N/255 for all 255 possible accuracy values.

**Examples:**

| Move accuracy | N | Our hit rate | Ideal (N/255) | Error |
|---|---|---|---|---|
| 100% | 255 | 100% (always hit) | 100% | 0 |
| 95% | 242 | 94.92% (≤, N≥128) | 94.90% | 0.02% |
| 85% | 216 | 84.77% (≤, N≥128) | 84.71% | 0.06% |
| 30% | 76 | 29.69% (<, N<128) | 29.80% | 0.12% |

Maximum error for any accuracy value: 1/(2×256) ≈ 0.20%, compared to 1/256 ≈ 0.39% without optimal rounding.

## Common jj Commands

### Navigation

```bash
jj log                              # View commit graph
jj log -r '::dashed-patch ~ ::master'  # Show only fork commits
jj status                           # Current state
jj diff                             # Changes in working copy
```

### Bookmarks (branches)

```bash
jj bookmark list                    # List bookmarks
jj bookmark create <name>           # Create at current commit
jj bookmark set <name> -r <rev>     # Move bookmark
jj bookmark set <name> --allow-backwards  # Move bookmark backward
```

### Editing history

```bash
jj edit <change-id>                 # Edit existing commit in-place
jj new                              # Create new commit
jj new <rev> -m "message"           # Create after specific commit
jj squash                           # Fold changes into parent
jj describe -m "message"            # Change commit message
jj abandon <change-id>              # Remove commit
```

### Rebasing

```bash
jj rebase -d master                 # Rebase current onto master
jj rebase -s <rev> -d <dest>        # Rebase rev and all descendants
jj rebase -r <rev> -d <dest>        # Rebase only rev (not descendants)
jj rebase -b <bookmark> -d <dest>   # Rebase branch onto dest
```

### Syncing with Git

```bash
jj git fetch                        # Fetch from all remotes
jj git fetch --remote author        # Fetch from upstream only
jj git push --tracked               # Push tracked bookmarks
jj git push --bookmark <name>       # Push specific bookmark
```

### Undo mistakes

```bash
jj undo                             # Undo last operation
jj op log                           # View operation history
jj op restore <op-id>               # Restore to specific state
```

---

*Last updated: 2026-05-23*
