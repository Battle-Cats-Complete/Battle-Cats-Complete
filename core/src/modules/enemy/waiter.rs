use std::fs;

use nyanko::enemy::unit::Battle;

use crate::Vfs;

pub(crate) fn t_unit(vfs: &Vfs, filename: &str) -> Option<Vec<Battle>> {
    let path = vfs.list(filename).into_iter().next()?;
    let bytes = fs::read(path).ok()?;

    Battle::parse_all(bytes).ok()
}
