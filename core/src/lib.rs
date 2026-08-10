#![warn(unreachable_pub)]
mod vault;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod addons;
pub mod animation;
pub mod common;
pub mod modules;
pub mod statblock;

pub use vault::{
    CatStore, Conflict, ContentStore, EnemyStore, Mount, StageStore, Target, Vault, Vds, Vfs, VfsError,
};
