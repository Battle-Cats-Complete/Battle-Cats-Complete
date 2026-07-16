pub mod abilities;
pub mod registry;

use nyanko::enemy::unit::Battle;

use crate::common::context::GlobalContext;
use crate::modules::enemy::game::registry::Magnification;

#[derive(Clone, Copy)]
pub struct EnemyRenderContext<'a> {
    pub global: GlobalContext<'a>,
    pub stats: &'a Battle,
    pub magnification: Magnification,
}