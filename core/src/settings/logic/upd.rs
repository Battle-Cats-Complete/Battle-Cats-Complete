use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum UpdateMode {
    AutoReset,
    AutoLoad,
    #[default]
    Prompt,
    Ignore,
}


impl UpdateMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::AutoReset => "Auto-Reset",
            Self::AutoLoad => "Auto-Load",
            Self::Prompt => "Prompt",
            Self::Ignore => "Ignore",
        }
    }
}