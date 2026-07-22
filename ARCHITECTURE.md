# Architecture

The Affective Engine is a runtime for deterministic simulations. Its organising
constraint is one sentence:

> **The simulation advances in whole, equal steps, and nothing the renderer does
> may change that.**

Everything else follows. Variable `dt` makes a run depend on frame timing, and a
run that depends on frame timing cannot be replayed, snapshotted, or lockstepped
across a network. So the engine steps the simulation on a fixed clock and hands
the renderer a *fraction* to draw with — never a partial step to simulate.

## Where this comes from

This engine is being extracted from a working game rather than designed in the
abstract. The proving ground is
[metadatastician/IDApTIK](https://github.com/metadatastician/IDApTIK), an
asymmetric two-player infiltration game whose Rust core already holds a
deterministic, event-sourced simulation with replay and snapshot tests.

The rule is that **a component earns its way in by working somewhere real
first.** Where something has served in IDApTIK, these docs say so; where it has
not, they say that too — because the second kind is where the bugs will be.
See `docs/decisions/` for the decisions behind each choice.

## Layout

```
.
├── crates/
│   └── affective-time/     # fixed-timestep stepping + render interpolation
│       ├── src/blend.rs    #   DoubleBuffer, Blend, lerp   (proven in IDApTIK)
│       └── src/step.rs     #   FixedStep accumulator        (new here)
├── docs/decisions/         # architecture decision records
├── docs/legal/             # licence notices
├── .machine_readable/      # A2ML manifests, contractiles, descriptiles
├── build/just/             # Justfile phases (imported by ./Justfile)
└── scripts/                # repo-shape and validation gates
```

## `affective-time`

The first crate, and the seam the rest of the engine will hang off.

**`DoubleBuffer<T, N>`** holds the last two simulated states in two inline
fixed-size arrays and samples between them. It does not allocate: `commit` is a
copy, there is no `Vec` and no `Box`, so neither the per-frame nor the per-step
path can stall on the allocator. A test asserts the size is exactly
`2 * N * size_of::<T>()`, so that claim fails a check rather than living in a
comment.

**`FixedStep`** converts real elapsed time into a whole number of owed steps plus
a fraction. It caps the steps one call may return and **discards** the excess, so
a stall makes the simulation run *slow* rather than spiral — trying to catch up
takes longer than real time, which makes the next frame later still, without
bound.

### The interpolation invariant

**Nothing produced by `DoubleBuffer` may flow back into simulation state.** It is
a pure function of two states that have already been simulated. This is the
property determinism rests on, and it is the first thing to check in any
integration.

Two corollaries that are easy to get wrong:

- **Only continuous quantities blend.** Booleans, enums, visibility and colour
  are read live from the current step. Sign-like values — a facing direction —
  are discrete in this sense even though they are stored as numbers, because
  lerping one through zero draws the subject facing neither way.
- **A discontinuity is `commit` then `snap`, never `snap` alone.** On a restart,
  teleport, level load or network resync the fresh state still has to enter the
  buffer; `snap` only discards the stale previous step. Snapping without
  committing keeps drawing the *old* position for another interval.

## Host integration

The engine does not own the frame loop; the host does. The shape is:

```rust
for _ in 0..clock.advance(real_dt) {
    world.step();                     // whole steps only
    buffer.commit(&world.poses());
}
draw(buffer.sample(slot, clock.alpha()));
```

The engine deliberately knows nothing about *what* a slot means. The host holds
its own indices and decides what a pose is. That boundary is what lets the same
buffers serve a 2D platformer, a network simulation and a terminal frontend
without the engine growing opinions about any of them.

## What is not here yet

Honesty is cheaper than surprise:

- **No renderer, no ECS, no asset pipeline, no audio, no input.** "Engine" here
  currently means the timing and interpolation core, and nothing more.
- **`FixedStep` has never run a real game.** IDApTIK borrows Bevy's accumulator,
  so this implementation is new code. It is tested hard — the spiral guard,
  hostile input, and a time-accounting invariant — but tests are not service.
- **AffineScript is the intended eventual implementation language; this is
  Rust.** See ADR-0004 for why, and for what would have to change.
