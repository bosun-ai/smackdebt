use crate::report::FileId;
use std::collections::BTreeMap;

macro_rules! directory_index {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u32);

        impl $name {
            /// Creates an index from a table position.
            pub const fn from_index(index: usize) -> Self {
                Self(index as u32)
            }

            /// Returns the table position represented by this index.
            pub const fn index(self) -> usize {
                self.0 as usize
            }

            /// Returns the compact integer representation.
            pub const fn get(self) -> u32 {
                self.0
            }
        }
    };
}

directory_index!(DirectoryId);

/// The directory tree over the repository-relative paths discovery produced.
///
/// The tree is a pure structural fact about those paths: it holds one node per
/// distinct directory, each node's parent and depth, and the directory of every
/// file. Directory distance is read from it as an integer, so no signal ever
/// compares two path strings to decide how far apart two files sit.
///
/// A node stores its own component name once, as the key its parent files it
/// under, so a tree over a repository costs one owned name per directory rather
/// than one clone per file path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryTree {
    parent: Vec<Option<DirectoryId>>,
    depth: Vec<u32>,
    children: Vec<BTreeMap<Box<str>, DirectoryId>>,
    file_directory: Vec<DirectoryId>,
}

impl DirectoryTree {
    /// The repository root, which every tree holds even when it holds no file.
    pub const ROOT: DirectoryId = DirectoryId::from_index(0);

    /// Builds the tree over repository-relative file paths, in one visit per
    /// path component, with the file order the paths arrive in as file identity.
    ///
    /// A path's last component is its file name, and an empty or `.` component
    /// names the directory it sits in, so the repository scope path `.` and a
    /// path holding no directory at all both land on the root.
    ///
    /// Paths are read through a borrowed iterator rather than an owned slice so
    /// that a caller holding inventory paths, report records, or history file
    /// entries builds the tree without cloning one path.
    pub fn from_file_paths<P: AsRef<str>>(paths: impl IntoIterator<Item = P>) -> Self {
        let paths = paths.into_iter();
        let mut tree = Self {
            parent: vec![None],
            depth: vec![0],
            children: vec![BTreeMap::new()],
            file_directory: Vec::with_capacity(paths.size_hint().0),
        };
        for path in paths {
            let path = path.as_ref();
            let mut directory = Self::ROOT;
            let mut components = path.split('/').peekable();
            while let Some(component) = components.next() {
                if components.peek().is_none() {
                    break;
                }
                if component.is_empty() || component == "." {
                    continue;
                }
                directory = tree.child(directory, component);
            }
            tree.file_directory.push(directory);
        }
        tree
    }

    /// The number of directories the tree holds, including the root.
    pub fn directory_count(&self) -> usize {
        self.parent.len()
    }

    /// The directory that holds this file, when the tree was built over it.
    pub fn directory_of(&self, file: FileId) -> Option<DirectoryId> {
        self.file_directory.get(file.index()).copied()
    }

    /// The directory this repository-relative directory path names, when the
    /// tree holds one.
    pub fn directory_of_path(&self, path: &str) -> Option<DirectoryId> {
        let mut directory = Self::ROOT;
        for component in path.split('/') {
            if component.is_empty() || component == "." {
                continue;
            }
            directory = *self.children[directory.index()].get(component)?;
        }
        Some(directory)
    }

    /// The directory that contains this one, which the root does not have.
    pub fn parent(&self, directory: DirectoryId) -> Option<DirectoryId> {
        self.parent[directory.index()]
    }

    /// The number of directories between this one and the root.
    pub fn depth(&self, directory: DirectoryId) -> u32 {
        self.depth[directory.index()]
    }

    /// The integer distance between two directories, which is
    /// `depth(left) + depth(right) - 2 x depth(lowest common ancestor)`.
    ///
    /// Two files of one directory are 0 apart and a directory and its direct
    /// parent are 1 apart. The distance is a property of the pair alone, so it
    /// never depends on the selected scope.
    pub fn distance(&self, left: DirectoryId, right: DirectoryId) -> u32 {
        let (mut lower, mut higher) = (left, right);
        let mut steps = 0;
        while self.depth(lower) > self.depth(higher) {
            lower = self.lift(lower);
            steps += 1;
        }
        while self.depth(higher) > self.depth(lower) {
            higher = self.lift(higher);
            steps += 1;
        }
        while lower != higher {
            lower = self.lift(lower);
            higher = self.lift(higher);
            steps += 2;
        }
        steps
    }

    /// The containing directories of this one, ordered root-ward and excluding
    /// the directory itself.
    pub fn ancestors(&self, directory: DirectoryId) -> impl Iterator<Item = DirectoryId> + '_ {
        let mut next = self.parent(directory);
        std::iter::from_fn(move || {
            let current = next?;
            next = self.parent(current);
            Some(current)
        })
    }

    /// The existing child of this directory with this component name, or a new
    /// one filed under it.
    fn child(&mut self, parent: DirectoryId, name: &str) -> DirectoryId {
        if let Some(&existing) = self.children[parent.index()].get(name) {
            return existing;
        }
        let child = DirectoryId::from_index(self.parent.len());
        self.parent.push(Some(parent));
        self.depth.push(self.depth(parent) + 1);
        self.children.push(BTreeMap::new());
        self.children[parent.index()].insert(name.into(), child);
        child
    }

    /// The parent of a directory the walk has already shown to have one.
    fn lift(&self, directory: DirectoryId) -> DirectoryId {
        self.parent(directory)
            .expect("a directory below the root has a parent")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds from borrowed paths; the ten thousand path case below builds from
    /// an owned slice, so both call shapes stay proven.
    fn tree(paths: &[&str]) -> DirectoryTree {
        DirectoryTree::from_file_paths(paths.iter().copied())
    }

    fn directory(tree: &DirectoryTree, file: usize) -> DirectoryId {
        tree.directory_of(FileId::from_index(file))
            .expect("the tree holds every file it was built from")
    }

    #[test]
    fn a_file_without_a_directory_component_sits_in_the_repository_root() {
        let tree = tree(&["README.md", "src/main.rs"]);
        assert_eq!(directory(&tree, 0), DirectoryTree::ROOT);
        assert_eq!(tree.depth(DirectoryTree::ROOT), 0);
        assert_eq!(tree.parent(DirectoryTree::ROOT), None);
        assert_eq!(tree.depth(directory(&tree, 1)), 1);
        assert_eq!(tree.directory_of(FileId::from_index(2)), None);
    }

    #[test]
    fn two_files_of_one_directory_are_zero_apart_and_a_child_is_one_step_from_its_parent() {
        let tree = tree(&["src/a.rs", "src/b.rs", "src/deep/c.rs"]);
        assert_eq!(directory(&tree, 0), directory(&tree, 1));
        assert_eq!(tree.distance(directory(&tree, 0), directory(&tree, 1)), 0);
        assert_eq!(tree.distance(directory(&tree, 0), directory(&tree, 2)), 1);
        assert_eq!(tree.distance(directory(&tree, 2), directory(&tree, 0)), 1);
    }

    #[test]
    fn two_sibling_directories_are_two_apart() {
        let tree = tree(&["src/left/a.rs", "src/right/b.rs"]);
        assert_eq!(tree.distance(directory(&tree, 0), directory(&tree, 1)), 2);
    }

    #[test]
    fn two_subtrees_meeting_at_the_root_add_their_depths() {
        let tree = tree(&["one/two/three/a.rs", "four/five/b.rs"]);
        assert_eq!(tree.depth(directory(&tree, 0)), 3);
        assert_eq!(tree.depth(directory(&tree, 1)), 2);
        assert_eq!(tree.distance(directory(&tree, 0), directory(&tree, 1)), 5);
        assert_eq!(tree.distance(directory(&tree, 0), DirectoryTree::ROOT), 3);
    }

    #[test]
    fn ancestors_are_listed_root_ward_and_exclude_the_directory_itself() {
        let tree = tree(&["one/two/three/a.rs"]);
        let deepest = directory(&tree, 0);
        let listed: Vec<u32> = tree.ancestors(deepest).map(|d| tree.depth(d)).collect();
        assert_eq!(listed, vec![2, 1, 0]);
        assert_eq!(tree.ancestors(DirectoryTree::ROOT).count(), 0);
    }

    #[test]
    fn a_directory_path_names_the_same_node_the_files_below_it_name() {
        let tree = tree(&["crates/analysis/src/lib.rs", "README.md"]);
        assert_eq!(tree.directory_of_path("."), Some(DirectoryTree::ROOT));
        assert_eq!(tree.directory_of_path(""), Some(DirectoryTree::ROOT));
        assert_eq!(
            tree.directory_of_path("crates/analysis/src"),
            Some(directory(&tree, 0))
        );
        assert_eq!(tree.directory_of_path("crates/output"), None);
    }

    #[test]
    fn the_tree_holds_one_node_per_distinct_directory_however_many_files_name_it() {
        let paths: Vec<String> = (0..10_000)
            .map(|file| format!("pkg{}/mod{}/file{}.rs", file / 100, file % 100, file))
            .collect();
        let tree = DirectoryTree::from_file_paths(&paths);
        // One root, one hundred package directories, and one directory per
        // package and module pair — every repeated component is visited once
        // and stored once.
        assert_eq!(tree.directory_count(), 1 + 100 + 100 * 100);
        assert_eq!(tree.depth(directory(&tree, 9_999)), 2);
        assert_eq!(tree.distance(directory(&tree, 0), directory(&tree, 1)), 2);
        assert_eq!(
            tree.distance(directory(&tree, 0), directory(&tree, 9_999)),
            4
        );
    }
}
