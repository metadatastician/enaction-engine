// SPDX-License-Identifier: AGPL-3.0-or-later
//! Creusot's deliberately narrow view of production Rust.
//!
//! The path module below is the exact file compiled by `enaction-accelerator`,
//! not a copied model. Add modules only after Creusot can translate their full
//! dependency surface; a harness compiling is not itself a proof claim.

#[path = "../../../crates/enaction-accelerator/src/version.rs"]
pub mod accelerator_version;

#[path = "../../../crates/enaction-contracts/src/version.rs"]
pub mod contracts_version;
