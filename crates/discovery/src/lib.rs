#![forbid(unsafe_code)]

mod glob;
mod inventory;
mod walk;

pub use glob::glob_matches;
pub use inventory::{
    DiscoveredFile, Inventory, Package, PackageId, RelativePath, SnapshotInventory,
    discover_snapshot, generic_source_roles, has_generated_javascript_name,
    has_vendored_javascript_name, is_runtime_javascript_path, is_source_path,
    is_tool_configuration_name,
};
