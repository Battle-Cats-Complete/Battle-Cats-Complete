use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Region {
    Ja,
    En,
    Tw,
    Ko,
}

#[derive(Clone, Copy, Debug)]
pub struct RegionMetadata {
    #[allow(dead_code)] pub internal_code: &'static str,
    pub package_suffix: &'static str,
    pub display_name: &'static str,
}

impl Region {
    pub const fn metadata(&self) -> RegionMetadata {
        match self {
            Region::Ja => RegionMetadata {
                internal_code: "ja",
                package_suffix: "",
                display_name: "Japan",
            },
            Region::En => RegionMetadata {
                internal_code: "en",
                package_suffix: "en",
                display_name: "Global",
            },
            Region::Tw => RegionMetadata {
                internal_code: "tw",
                package_suffix: "tw",
                display_name: "Taiwan",
            },
            Region::Ko => RegionMetadata {
                internal_code: "ko",
                package_suffix: "kr",
                display_name: "Korea",
            },
        }
    }

    #[allow(dead_code)]
    pub fn from_str(input_string: &str) -> Option<Self> {
        match input_string.to_lowercase().as_str() {
            "ja" | "jp" | "battlecats" => Some(Region::Ja),
            "en" => Some(Region::En),
            "tw" => Some(Region::Tw),
            "ko" | "kr" => Some(Region::Ko),
            _ => None,
        }
    }
}