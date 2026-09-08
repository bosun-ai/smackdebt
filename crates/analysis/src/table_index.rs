//! Private shared definition of compact report-table identities.

macro_rules! table_index {
    ($(#[$documentation:meta])* $name:ident) => {
        $(#[$documentation])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u32);

        impl $name {
            /// Creates an identity from a table position.
            pub const fn from_index(index: usize) -> Self {
                Self(index as u32)
            }
            /// Returns the table position represented by this identity.
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

pub(crate) use table_index;
