// SPDX-License-Identifier: MPL-2.0
//! Pointwise binary32 kernels adapted from Axiom.jl.
//!
//! Provenance: `zig/src/activations.zig` at Axiom.jl commit
//! 240a5d7fa1d5e4646d8d0970ee7b7581edda1e16. The estate ABI wrapper adds
//! finite-input, length, aliasing, lane, and failure-atomicity checks before
//! entering these kernels. Axiom remains the upstream source for this file's
//! covered implementation; the surrounding ABI and dispatch code is AGPL.

const vector_size = 8;
const Vector = @Vector(vector_size, f32);

/// ReLU with a canonical positive-zero result for every non-positive input.
pub fn relu(input: []const f32, output: []f32) void {
    const zero: Vector = @splat(0.0);
    var index: usize = 0;
    while (index + vector_size <= input.len) : (index += vector_size) {
        const values: Vector = input[index..][0..vector_size].*;
        output[index..][0..vector_size].* = @select(f32, values > zero, values, zero);
    }
    while (index < input.len) : (index += 1) {
        output[index] = if (input[index] > 0.0) input[index] else 0.0;
    }
}

/// ReLU6 with canonical positive zero and an inclusive upper clamp.
pub fn relu6(input: []const f32, output: []f32) void {
    const zero: Vector = @splat(0.0);
    const six: Vector = @splat(6.0);
    var index: usize = 0;
    while (index + vector_size <= input.len) : (index += vector_size) {
        const values: Vector = input[index..][0..vector_size].*;
        const positive = @select(f32, values > zero, values, zero);
        output[index..][0..vector_size].* = @select(f32, positive < six, positive, six);
    }
    while (index < input.len) : (index += 1) {
        output[index] = if (input[index] <= 0.0)
            0.0
        else if (input[index] >= 6.0)
            6.0
        else
            input[index];
    }
}
