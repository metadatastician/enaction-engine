// SPDX-License-Identifier: MPL-2.0
//! Binary32 matrix multiplication adapted from Axiom.jl.
//!
//! Provenance: `zig/src/axiom.zig` at Axiom.jl commit
//! 38cfdd6. The estate ABI wrapper supplies all validation and performs a
//! finite-result preflight before entering this allocation-free kernel.

pub fn matmul(m: usize, k: usize, n: usize, left: []const f32, right: []const f32, output: []f32) void {
    for (0..m) |row| {
        for (0..n) |column| {
            var sum: f32 = 0.0;
            for (0..k) |inner| sum += left[row * k + inner] * right[inner * n + column];
            output[row * n + column] = sum;
        }
    }
}
