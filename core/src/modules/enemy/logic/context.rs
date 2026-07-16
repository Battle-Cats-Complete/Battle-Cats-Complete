use nyanko::enemy::unit::Battle;

use crate::modules::enemy::registry::Magnification;
use crate::common::context::GlobalContext;

#[derive(Clone, Copy)]
pub struct EnemyRenderContext<'a> {
    pub global: GlobalContext<'a>,
    pub stats: &'a Battle,
    pub magnification: Magnification,
}