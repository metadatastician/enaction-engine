// SPDX-License-Identifier: AGPL-3.0-or-later
//! Accelerator contract-version compatibility.

use creusot_std::macros::ensures;
use creusot_std::prelude::DeepModel;

/// A two-part compatibility version. A host accepts the same major and an
/// operation minor version no newer than itself.
#[derive(Clone, Copy, Debug, DeepModel)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq, PartialOrd, Ord))]
pub struct ContractVersion {
    pub major: u16,
    pub minor: u16,
}

impl ContractVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    /// Creusot checks deductively that the implementation returns exactly
    /// "same major and no newer minor".
    #[ensures(result == (self.major == requested.major && requested.minor <= self.minor))]
    pub const fn accepts(self, requested: Self) -> bool {
        self.major == requested.major && requested.minor <= self.minor
    }
}
