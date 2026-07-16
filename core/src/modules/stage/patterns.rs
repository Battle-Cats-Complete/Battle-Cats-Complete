pub const MAP_STAGE_DATA_PATTERN: &str = r"^MapStageData([a-zA-Z]+)_(\d+)\.csv$";
pub const MAP_NAME_PATTERN: &str = r"^mapname(\d+)_([a-zA-Z]{1,3})(?:_[a-z]{2})?\.(png|imgcut|maanim|mamodel)$";
pub const MAP_SN_PATTERN: &str = r"^mapsn(\d+)_(\d+)_([a-zA-Z]{1,3})(?:_[a-z]{2})?\.(png|imgcut|maanim|mamodel)$";

pub const MAP_GLOBAL_NAME_PATTERN: &str = r"^Map_Name(?:_[a-z]{2})?\.csv$";

pub const STAGE_NORMAL_PATTERN: &str = r"^stageNormal(\d)(?:_(\d))?(?:_Invasion)?(?:_Z)?\.csv$";

pub const STAGE_FILE_PATTERN: &str = r"^stage([a-zA-Z]+)?(\d+)(?:_Invasion)?(?:_Z)?(?:_(\d+))?\.csv$";
pub const STAGE_NAME_PATTERN: &str = r"^StageName_([a-zA-Z]+)(?:_[a-z]{2})?\.csv$";
pub const STAGE_NAME_NUMERIC_PATTERN: &str = r"^StageName([0-2])(?:_[a-z]{2})?\.csv$";
pub const STAGE_NAME_BASE_PATTERN: &str = r"^StageName(?:_[a-z]{2})?\.csv$";
pub const STAGE_FIRST_MESSAGE_PATTERN: &str = r"^StageFirstMessage(?:_[a-z]{2})?\.csv$";

pub const LEGACY_STAGE_NAME_PATTERN: &str = r"^([a-zA-Z][cC])(\d+)_([a-zA-Z]{1,3})(?:_[a-z]{2})?\.(png|imgcut|maanim|mamodel)$";

pub const CASTLE_PATTERN: &str = r"^(?:castle_)?([a-zA-Z][cC])(\d+)(?:_\d+)?(?:_[a-z]{2})?\.(png|imgcut|maanim|mamodel)$";

pub const BG_MAP_PATTERN: &str = r"^map(\d+)(?:_[0-9_]+)?\.(png|imgcut|maanim|mamodel|json)$";
pub const BG_BATTLE_PATTERN: &str = r"^bg(\d+)(?:_[0-9_]+)?\.(png|imgcut|maanim|mamodel)$";
pub const BG_DATA_PATTERN: &str = r"^bg(\d+)(?:_[0-9_]+)?\.json$";
pub const BG_EFFECT_PATTERN: &str = r"^bgEffect_(\d+)(?:_[0-9_]+)?\.(png|imgcut|maanim|mamodel|json)$";

pub const LIMIT_MSG_PATTERN: &str = r"^MapStageLimitMessage(?:_[a-z]{2})?\.csv$";
pub const EX_PATTERN: &str = r"^EX_(group|lottery|option)\.csv$";
pub const CERTIFICATION_PRESET_PATTERN: &str = r"^Certification(\d+)\.preset$";
pub const DROP_ITEM_PATTERN: &str = r"^DropItem(?:_[a-z]{2})?\.csv$";
pub const CHARAGROUP_PATTERN: &str = r"^Charagroup(?:_[a-z]{2})?\.csv$";
pub const SCORE_BONUS_PATTERN: &str = r"^ScoreBonusMap(?:_[a-z]{2})?\.json$";
pub const DIFFICULTY_LEVEL_PATTERN: &str = r"^difficulty_level(?:_[a-z]{2})?\.tsv$";
pub const DROP_CHARA_PATTERN: &str = r"^drop_chara(?:_[a-z]{2})?\.csv$";
pub const LOCK_SKIP_DATA_PATTERN: &str = r"^LockSkipData\.csv$";
pub const SCAT_CPU_SETTING_PATTERN: &str = r"^ScatCPUsetting\.csv$";