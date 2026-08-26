#![warn(unreachable_pub)]
mod vault;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod common;
pub mod domains;
pub mod systems;

pub use vault::{
    CatStore, Conflict, ContentStore, EnemyStore, ItemStore, Listing, Mount, StageStore, Target, Vault, Vds, Vfs,
    VfsError,
};
