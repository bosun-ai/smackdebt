#![forbid(unsafe_code)]

mod glob;
mod inventory;
mod walk;

pub use glob::glob_matches;
pub use inventory::{
    DiscoveredFile, Inventory, Package, PackageId, RelativePath, SnapshotInventory,
    discover_snapshot, generic_source_roles, is_source_path,
};
