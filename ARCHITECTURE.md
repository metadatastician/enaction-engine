<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# Enaction Engine architecture

Enaction Engine is a deterministic, type-safe game engine for worlds shaped
through perception, affect, intention, action and consequence.

Its first invariant is:

> The simulation advances in whole, equal steps, and render interpolation never
> feeds simulation state.

Today, only the first part of the deterministic substrate exists as the Rust
crate `enaction-time`. The rest of this document is a target architecture, not
an inventory of implemented features.

## Layered destination

```text
Enaction Engine
├── deterministic substrate
├── world and state model
├── agency and embodiment
├── cognition
├── affect
├── conation
├── behaviour and action
├── physics and interaction
├── presentation
├── networking and persistence
└── tooling and host integration
```

The destination also encompasses input, sound and music, assets and content,
rendering, multiplayer, replay, and released-game host contracts. These
capabilities enter only when backed by real implementation and tests.

## Dependency direction

The architectural boundaries are directional:

* Low-level deterministic facilities know nothing about game content or domain
  ontology.
* World and state facilities provide stable host-defined entities, events, and
  queries without assuming one game's rules.
* Cognition, affect, and conation are sibling dimensions that operate over
  host-defined entities and events. None names or owns the whole engine.
* Behaviour and action turn situated state into constrained action; games own
  their ontology, semantics, and game-specific rules.
* Render interpolation consumes completed simulation states and must never feed
  a value back into simulation.
* Presentation and tools depend on runtime contracts, not the reverse.
* Universal Modding Studio authors and validates content but is not required by
  a released game. No circular dependency is allowed.

## Implemented deterministic substrate

```text
crates/enaction-time/
├── src/blend.rs   DoubleBuffer, Blend, endpoint-exact lerp
├── src/step.rs    FixedStep accumulator and catch-up cap
└── tests/         timing, interpolation, discontinuity, and layout evidence
```

`FixedStep` converts elapsed wall time into a count of whole simulation steps
and a render-only remainder. It caps catch-up and exposes when excess time was
dropped. This implementation is new here and has not run in a real game.

`DoubleBuffer<T, N>` holds two completed states in fixed-size inline arrays.
`sample` interpolates continuous values for display; `curr` exposes unblended
current values for discrete presentation. Its design and edge cases derive from
IDApTIK experience.

A discontinuity is handled by committing the fresh state and then snapping the
history. Snapping alone cannot introduce the new state.

The current code does not prove whole-game determinism. A host must also control
input ordering, state representation, arithmetic, randomness, concurrency, and
external effects.

## Host boundary

The host owns the frame loop and domain state:

```rust
for _ in 0..clock.advance(real_dt) {
    world.step();                      // whole fixed step
    poses.commit(&world.poses());
}

draw(poses.sample(slot, clock.alpha())); // presentation only
```

The engine does not assign semantic meaning to a buffer slot. Future reusable
host contracts should expose versioned, typed events and state views without
forcing games into an engine-owned domain ontology.

## Cognition, affect, and conation

Cognition includes perception, attention, memory, belief, inference, and
planning. Affect includes appraisal, mood, atmosphere, trust, fear, attachment,
and significance. Conation includes needs, motives, goals, commitment,
inhibition, and action selection.

These dimensions interact through an agent situated in a world, but they remain
separable subsystem families. Agency and embodiment connect their state to
possible action; worlds, ecologies, and institutions supply constraints and
consequences.

### What is implemented (`enaction-trace`)

The **seam** exists, and part of the affective and epistemic content with it.
The engine supplies identity, ordering, causality and domain separation; a game
supplies every word.

**Typed by domain.** `Domain` separates `Mechanical`, `Epistemic`, `Affective`,
`Conative`, `BehaviouralSelection` and `SocialRelational`. Affect never
silently becomes knowledge, and nothing selects behaviour without an explicit
selection event. `domain` lives on the trope rather than the event, so a
particular cannot be reclassified mid-history.

**Particularised by trope.** A `Trope` is an *abstract particular* — this
bearer's this property-instance, with identity (Williams/Campbell: properties
are particulars, not universals). Two guards suspecting the same object are two
particulars with independent causal histories that merely *resemble* one
another; `resembles` / `is_peer_of` / `peers_of` make that a query rather than
an equality test, which is what `SocialRelational` reasoning is built over.

**Theory of mind by ascription path.** `Trope::holders` records whose model a
particular lives inside, outermost first: `[]` is ground truth, `["billy"]` is
Billy's model of the bearer's state, `["billy","anya"]` is Billy's model of
Anya's model. Dennett's orders are path length; the shared ESM contract's
declared depth bound is `holders.len()`, enforced by `DEFAULT_MAX_TOM_ORDER`.

It is a **path, not a reference**, and that is the design. An ascription must
be able to describe a state that does not exist — an agent may model a belief
no one holds. A reference would make false ascription unrepresentable, and
being wrong about other minds is the phenomenon the field exists to carry. From
it: `divergence()` (how wrong a model is, in milliunits) and `projections()`
(states an agent models in someone who lacks them). The **Sally-Anne
false-belief task runs as a test**.

**Interest and belief kinetics** (`enaction-trace::belief`), extracted from
IDApTIK with bit-for-bit parity: meters climb per observation channel, decay
while unobserved unless *pinned*, and belief forms over the highest once it
crosses a threshold. Observation channels are caller-supplied data
(`ChannelTable`), and the advance parameter is any monotone accumulator —
seconds for IDApTIK, steps-walked for Chronicles' "memory erodes with motion".

**Not yet implemented:** the belief store (baseline plus per-bearer deltas),
the bounded confidence type, retraction/revision events, valence, goal and
commitment machinery, and behavioural-selection chain enforcement. No game
consumes any of this yet.

## Proving-ground policy

IDApTIK is the first proving ground. Its working simulation supplies experience
and candidate seams, but extraction must remove game-specific assumptions.
Chronicles of Slavia is the second planned abstraction test: a boundary is not
general merely because the first game can use it.

New components must earn entry through a real use, an implemented seam, tests,
and honest provenance. Speculative empty crates are prohibited.

## UMS and released games

UMS Studio and UMS Core sit on the authoring side. A versioned game profile
describes valid content; an optional Enaction adapter may execute previews and
tests. The output is a validated/compiled game package consumed by the released
game. The runtime never imports the editor application, and UMS Core is not
hard-wired to this engine. See
[`docs/architecture/UMS-INTEGRATION.adoc`](docs/architecture/UMS-INTEGRATION.adoc).

## Current limits

- **No renderer, no ECS, no asset pipeline, no audio, no input.** "Engine" here
  means the deterministic timing/interpolation substrate plus the cognitive
  trace seam — and nothing more.
- **Nothing consumes the cognition seam yet.** The trope, belief and
  theory-of-mind machinery is tested but unadopted; a component in use is
  extracted, a component merely present is copied. IDApTIK adoption is
  sequenced after its own in-flight work; Chronicles is the generalisation
  test.
- **`FixedStep` has never run a real game.** IDApTIK borrows Bevy's accumulator,
  so this implementation is new code. It is tested hard — the spiral guard,
  hostile input, and a time-accounting invariant — but tests are not service.
- **AffineScript is the intended eventual implementation language; this is
  Rust.** See ADR-0004 for why, and for what would have to change.

## UMS and game boundary

Enaction is below game runtimes in the dependency graph. It never imports UMS,
game profiles or editor vocabulary. UMS may eventually use an optional adapter
for preview, but that adapter must translate editor data into public Enaction
inputs; it must not reverse the dependency.

The current evidence is deliberately narrow:

- `DoubleBuffer`, `Blend` and endpoint-exact interpolation were extracted from
  IDApTIK and have run in its Bevy frontend.
- `crates/enaction-time/tests/idaptik_parity.rs` compares the extraction
  boundary for fixed-step accounting, interpolation, discontinuities, hostile
  elapsed time, discrete versus continuous values, and restart/snapshot
  interaction.
- IDApTIK has not adopted this repository's `FixedStep`; it continues to use
  Bevy's fixed clock. No replacement is justified yet.
- `enaction-trace` is new, game-neutral and has not run in either game.
  IDApTIK currently uses a local game-vocabulary trace with the same six-domain
  separation while the general seam remains revisable.

Nothing has been extracted from Slavia in this pass. The UMS Slavia profile
identifies candidate future primitives—receptive fields, place memory,
relationships and influence—but those are designs, not engine components.
