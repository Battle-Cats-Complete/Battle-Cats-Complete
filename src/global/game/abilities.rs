#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum CustomIcon {
    #[default] None,
    Multihit,
    Kamikaze,
    BossWave,
    Dojo,
    StarredAlien,
    Burrow,
    Revive,
    Stop,
    DeathTimer,
    God,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct AbilityItem {
    pub icon_id: Option<usize>,
    pub text: String,
    pub custom_icon: CustomIcon,
    pub border_id: Option<usize>,
}

// UI Spacing Constants
pub const ABILITY_X: f32 = 3.0;
pub const ABILITY_Y: f32 = 5.0;
pub const TRAIT_Y: f32 = 7.0;