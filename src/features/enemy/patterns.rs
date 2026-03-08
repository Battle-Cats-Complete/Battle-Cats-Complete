// Matches: t_unit.csv
pub const ENEMY_STATS: &str = r"^t_unit\.csv$";

// Matches: enemy_icon_000.png
pub const ENEMY_ICON: &str = r"^enemy_icon_(\d{3})\.png$";

// Matches: 000_e.imgcut, 000_e.mamodel, 000_e.png
pub const ENEMY_ANIM_BASE: &str = r"^(\d{3})_e\.(imgcut|mamodel|png)$";

// Matches: 000_e00.maanim through 03, and 000_e_zombie00.maanim through 02
pub const ENEMY_MAANIM: &str = r"^(\d{3})_e(0[0-3]|_zombie0[0-2])\.maanim$";