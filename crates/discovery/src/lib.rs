#![forbid(unsafe_code)]

mod ignore;
mod inventory;

pub use inventory::{DiscoveredFile, Inventory, Package, PackageId, RelativePath};
