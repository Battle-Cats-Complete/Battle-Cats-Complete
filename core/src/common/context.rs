use nyanko::common::data::Localizable;
use nyanko::common::data::Param;

#[derive(Clone, Copy)]
pub struct GlobalContext<'a> {
    pub param: &'a Param,
    pub localizable: &'a Localizable,
}