# Progress

**Current section:** 3
**Last session:** 2026-11-03

## Sections

| # | Status | Gate | Date | ES synced | Notes |
|---|---|---|---|---|---|
| 1 | done | passed after misses (dangling raw pointer, local drop order, stdout/stderr redirect) — checkpoint code by Claude, explained back | 2026-11-04 | yes | Owner self-checked vs ANSWERS first; gate re-run with fresh questions |
| 2 | done | passed; explain-back corrected: trait signatures are fixed, `From` impl (not App) does error conversion | 2026-11-04 | yes | Checkpoint code by Claude (winit 0.30.13); verified on Mac (Esc, keys, resize) |
| 3–19 | not written | — | | — | Written on arrival |

## Prerequisite audit (2026-11-02)
- Rust: new. C/C++ and pointers: known.
- 3D rendering, GPU APIs, 3D maths: new (all 8 diagnostic questions "no idea").
- Windows/events: known via raylib and mlx.
- Maths depth: capped at T1 by owner's choice — explain the logic, use with ease.

## Overrides
- 2026-11-03: method changed — Claude writes checkpoint code after each gate, owner explains it back (see DECISIONS.md).

## Review queue

| Question | Source | Clean recalls | Next due |
|---|---|---|---|
| Q-1.5 (dangling CString pointer — ask a fresh variant) | § 1 | 0 | after § 3 |
| Stdout vs stderr redirect (`>` vs `2>`) — fresh variant | § 1 | 0 | after § 3 |
| Who converts errors for `?` (`From` impl) — fresh variant | § 2 | 0 | after § 4 |
| Q-1.3 (drop order fields vs locals — fresh variant) | § 1 | 0 | after § 3 |

## Open threads
- 2026-11-04: Spanish (learn_es/) paused from § 3 on, by owner's choice — catch up all sections at the end. `check_learn.py` parity errors for § 3+ are expected until then.
- § 1 weak spots: dangling raw pointer from a dropped local; drop order of locals vs struct fields.
- Confirm `vkcube` runs on a 42 Madrid campus machine (evaluation environment).
- Check the Vulkan version MoltenVK reports (`vulkaninfo --summary`) before § 5 —
  decides dynamic rendering vs render passes.
- Bonus extras (§ 19): deliberately undecided.
