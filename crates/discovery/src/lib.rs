#![forbid(unsafe_code)]

mod ignore;
mod inventory;

pub use ignore::glob_matches;
pub use inventory::{
    DiscoveredFile, Inventory, Package, PackageId, RelativePath, generic_source_roles,
};
