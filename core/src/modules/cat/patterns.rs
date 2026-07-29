pub(crate) const CAT_STATS_PATTERN: &str = concat!(r"^unit", r"(\d{3})", r"\.csv$");
pub(crate) const CAT_ICON_PATTERN: &str = concat!(r"^uni", r"(\d{3})_", r"([fcsud])", r"00\.png$");
pub(crate) const CAT_UPGRADE_PATTERN: &str = concat!(r"^udi", r"(\d{3})_", r"([fcsu])", r"\.png$");
pub(crate) const CAT_GACHA_PATTERN: &str = concat!(r"^gatyachara_", r"(\d{3})", r"_[fz]\.png$");
pub(crate) const CAT_ANIM_PATTERN: &str = concat!(r"^", r"(\d{3})_", r"([fcsu])", r"\.(imgcut|mamodel|png)$");
pub(crate) const CAT_MAANIM_PATTERN: &str = concat!(r"^", r"(\d{3})_", r"([fcsu])", r"(0[0-4]|_entry)\.maanim$");
pub(crate) const CAT_EXPLAIN_PATTERN: &str = r"^Unit_Explanation(\d+)\.csv$";

pub(crate) const EGG_ICON_PATTERN: &str = r"^uni(\d{3})_(m0[0-1])\.png$";
pub(crate) const EGG_UPGRADE_PATTERN: &str = r"^udi(\d{3})_(m0[0-1])\.png$";
pub(crate) const EGG_GACHA_PATTERN: &str = r"^gatyachara_(\d{3})_m\.png$";
pub(crate) const EGG_ANIM_PATTERN: &str = r"^(\d{3})_m\.(imgcut|mamodel|png)$";
pub(crate) const EGG_MAANIM_PATTERN: &str = r"^(\d{3})_m(0[0-3]|_zombie0[0-2])\.maanim$";

pub(crate) const SKILL_DESC_PATTERN: &str = r"^SkillDescriptions\.csv$";
pub(crate) const SKILL_NAME_PATTERN: &str = r"^Skill_name_\d{3}(?:_([a-z]{2}))?\.png$";

pub(crate) const CAT_EVOLVE_PATTERN: &str = r"^unitevolve\.csv$";

pub(crate) const CAT_UNIVERSAL_FILES: &[&str] = &[
    "SkillAcquisition.csv",
    "SkillLevel.csv",
    "unitbuy.csv",
    "unitexp.csv",
    "unitlevel.csv",
    "unitlimit.csv",
    "uni.png"
];