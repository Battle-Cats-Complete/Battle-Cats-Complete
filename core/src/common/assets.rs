use super::game::abilities::CustomIcon;

pub const MULTIHIT: &[u8] = include_bytes!("../assets/multihit.png");
pub const KAMIKAZE: &[u8] = include_bytes!("../assets/kamikaze.png");
pub const BOSS_WAVE: &[u8] = include_bytes!("../assets/boss_wave_immune.png");
pub const DOJO: &[u8] = include_bytes!("../assets/dojo.png");
pub const STARRED_ALIEN: &[u8] = include_bytes!("../assets/starred_alien.png");
pub const BURROW: &[u8] = include_bytes!("../assets/burrow.png");
pub const REVIVE: &[u8] = include_bytes!("../assets/revive.png");
pub const UDI_F: &[u8] = include_bytes!("../assets/udi_f.png");
pub const STOP: &[u8] = include_bytes!("../assets/stop_attack.png");
pub const DEATH_TIMER: &[u8] = include_bytes!("../assets/death_timer.png");
pub const GOD: &[u8] = include_bytes!("../assets/god.png");
pub const UNKNOWN: &[u8] = include_bytes!("../assets/unknown.png");

pub const ICON: &[u8] = include_bytes!("../assets/icon.ico");
pub const FONT_JP: &[u8] = include_bytes!("../assets/NotoSansJP-Regular.ttf");
pub const FONT_KR: &[u8] = include_bytes!("../assets/NotoSansKR-Regular.ttf");
pub const FONT_TC: &[u8] = include_bytes!("../assets/NotoSansTC-Regular.ttf");
pub const FONT_TH: &[u8] = include_bytes!("../assets/NotoSansThai-Regular.ttf");

pub const CUSTOM_ICON_DATA: &[(CustomIcon, &[u8])] = &[
    (CustomIcon::Multihit, MULTIHIT),
    (CustomIcon::Kamikaze, KAMIKAZE),
    (CustomIcon::BossWave, BOSS_WAVE),
    (CustomIcon::Dojo, DOJO),
    (CustomIcon::StarredAlien, STARRED_ALIEN),
    (CustomIcon::Burrow, BURROW),
    (CustomIcon::Revive, REVIVE),
    (CustomIcon::Stop, STOP),
    (CustomIcon::DeathTimer, DEATH_TIMER),
    (CustomIcon::God, GOD),
    (CustomIcon::Unknown, UNKNOWN),
];