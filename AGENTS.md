# AGENTS.md — Enaction Engine

Canonical, tool-agnostic instructions for every coding agent working in this
repository. `CLAUDE.md` carries the estate-wide RSR protocol; this file carries
what is specific to *this* repo. Where they conflict, this file wins.

## What this is

Enaction Engine is a deterministic, type-safe game engine for worlds shaped
through perception, affect, intention, action and consequence.

The current implementation is only its first deterministic substrate:
fixed-timestep stepping plus render interpolation. There is no renderer, ECS,
asset pipeline, audio, input, physics, networking, or complete cognitive,
affective, or conative subsystem. One crate exists: `enaction-time`.

Read `ARCHITECTURE.md` and `docs/decisions/` before proposing anything
structural.

## `[CRITICAL] agpl-license`

**Code is `AGPL-3.0-or-later`. Prose documentation is `CC-BY-SA-4.0`.**

This repository was minted from the RSR template, which ships **MPL-2.0**, and
was **deliberately relicensed at mint** so that code can move freely between
this repo and `metadatastician/IDApTIK` (also AGPL) without relicensing
friction. That relicensing is intentional and recorded in the initial commit.

Therefore:

- **Never rewrite this repository's licence headers back to MPL-2.0.** An
  estate-wide MPL-normalisation sweep has already wrongly flattened two AGPL
  projects in this estate, each needing manual restoration. If a sweep or tool
  proposes MPL headers here, **that is the bug** — stop and raise it rather than
  complying.
- The root `LICENSE` must remain the **full** AGPL-3.0 text. An SPDX stub makes
  GitHub report the repository as `NOASSERTION` instead of `AGPL-3.0`.
- `LICENSES/` holds the full text of every licence in use — currently
  `AGPL-3.0-or-later.txt` and `CC-BY-SA-4.0.txt`. `MPL-2.0.txt` was removed
  deliberately because nothing here is MPL any more; do not restore it.
- Check **bodies, not just SPDX lines**. Header/body mismatches have bitten this
  estate in both directions. `docs/legal/SOURCE-NOTICE.txt` exists precisely
  because the template's MPL "Exhibit A" has no AGPL equivalent.

## Design invariants

These are not style preferences; breaking any of them is a defect.

1. **The simulation advances only in whole, equal steps.** Never hand a partial
   step to simulation code. Determinism — and therefore replay, snapshots and
   lockstep networking — rests entirely on this.
2. **Interpolation is render-only.** No value produced by `DoubleBuffer` may flow
   back into simulation state.
3. **Only continuous quantities blend.** Booleans, enums and visibility are read
   live from the current step. A *facing direction* counts as discrete despite
   being a number: lerping a sign through zero draws the subject facing neither
   way.
4. **A discontinuity is `commit` then `snap`, never `snap` alone.** The fresh
   state must enter the buffer; `snap` only discards the stale previous step.
5. **Nothing on the per-frame or per-step path allocates.** `DoubleBuffer` is two
   inline fixed-size arrays. No `Vec`, no `Box`.

Rules 3–5 each came from a real bug and each is pinned by a test. If a test that
looks redundant is blocking a "simplification", read the test's comment before
deleting it — several exist specifically to document a wrong version.

## Float behaviour — do not "simplify" `lerp`

Two measured results shape it, and both are counter-intuitive:

- **Endpoints are clamped, not computed.** `prev + (curr - prev) * 1.0` is
  inexact: `(-5.5, 3.3)` → `3.3000000000000007`, `(1e16, 1.0)` → `0.0`.
- **The interior uses the one-term form.** `prev * (1 - alpha) + curr * alpha`
  looks tidier but returns `42.00000000000001` for a *stationary* value, so it
  shimmers every frame.

## Working rules

- **No fake checks.** A check that cannot fail is not a check. The RSR template
  shipped a `just test` that echoed "Tests passed!" without running anything;
  it was replaced at mint. Do not reintroduce that pattern, and do not add a
  gate that exits 0 when its tool is missing — an absent toolchain is a failure,
  not a skip.
- **Claim provenance honestly.** `DoubleBuffer` is proven in IDApTIK;
  `FixedStep` is new here and has never run a real game. Docs say so on purpose.
  Do not upgrade that language without the service to back it.
- **Do not create speculative subsystem crates.** A component needs real
  implementation, tests, and a proving-ground use or unavoidable substrate
  requirement before it enters the workspace.
- **Keep dimensions distinct.** Cognition, affect and conation are sibling
  dimensions of agency. None names or owns the entire engine, and host games
  retain their domain ontology and game-specific rules.
- **Keep UMS at the contract boundary.** Enaction Engine is runtime
  infrastructure. It must not depend on the UMS application, released games
  consume compiled packages rather than editor UI, and cycles are forbidden.
- **AffineScript is the eventual target, not the current language** (ADR-0004).
  Do not add it to the build.

## Verification

```
just test          # cargo test --workspace
just verify        # RSR gates
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
bash scripts/check-root-shape.sh
```
