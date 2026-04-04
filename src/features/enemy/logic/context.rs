use crate::features::enemy::data::t_unit::EnemyRaw;
use crate::features::enemy::registry::Magnification;
use crate::global::ui::shared::GlobalContext;

#[derive(Clone, Copy)]
pub struct EnemyRenderContext<'a> {
    pub global: GlobalContext<'a>,
    pub stats: &'a EnemyRaw,
    pub magnification: Magnification,
}