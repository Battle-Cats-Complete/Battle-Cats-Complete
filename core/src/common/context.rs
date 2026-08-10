use nyanko::common::data::Localizable;
use nyanko::common::data::Param;

use crate::Store;

#[derive(Clone, Copy)]
pub struct GlobalContext<'a> {
    pub param: &'a Param,
    pub localizable: &'a Localizable,
    pub store: &'a Store,
}