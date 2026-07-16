pub const ENEMY_STATS: &str = r"^t_unit\.csv$";

pub const ENEMY_ICON: &str = r"^enemy_icon_(\d{3})\.png$";

pub const ENEMY_ANIM_BASE: &str = r"^i?(\d{3})_e\.(imgcut|mamodel|png)$";

pub const ENEMY_MAANIM: &str = r"^(\d{3})_e(0[0-3]|_zombie0[0-2])\.maanim$";

pub const ENEMY_NAME: &str = r"^Enemyname(?:_([a-z]{2}))?\.tsv$";

pub const ENEMY_PICTURE_BOOK: &str = r"^EnemyPictureBook(?:_([a-z]{2}))?\.csv$";
pub const ENEMY_PICTURE_BOOK_2: &str = r"^EnemyPictureBook2(?:_([a-z]{2}))?\.csv$";
pub const ENEMY_PICTURE_BOOK_QUESTION: &str = r"^EnemyPictureBookQuestion(?:_([a-z]{2}))?\.csv$";

pub const ENEMY_DICT_LIST: &str = r"^enemy_dictionary_list\.csv$";
pub const AUTOSET_EXCLUDE: &str = r"^autoset_exclude_enemy\.csv$";

pub const ENEMY_ZOMBIE_EFFECT: &str = r"^set_enemy001_zombie(?:_[a-z]+)?\.(imgcut|mamodel|png|maanim)$";