use nyanko::common::data::Localizable;
use nyanko::common::data::Param;

use crate::Vault;

#[derive(Clone, Copy)]
pub struct GlobalContext<'a> {
    pub param: &'a Param,
    pub localizable: &'a Localizable,
    pub vault: &'a Vault,
}