use md5;
use nyanko::common::tools::variant::Region;
use serde::{Deserialize, Serialize};

pub const EXPECTED_HASHES: [(&str, &str); 4] = [
    ("bac299d3cf278544782427ff7c71ef58", "6910fae125547fd957a505c67e1c72bd"),
    ("b9e48b02312e5b3dd60194a03157d70c", "45cad482726268e341f5759230ce8cff"),
    ("264a0ffd5f69d257284b93ae881ce2b6", "213cecb58af008964303ecb2cf0f5373"),
    ("3d22eafdcc4fc2a1379b103970b36217", "4cacdb0839634116caaf0b966638865b"),
];

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Debug)]
pub struct RegionKey {
    pub key: String,
    pub iv: String,
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Debug)]
pub struct UserKeys {
    #[serde(alias = "jp")]
    pub ja: RegionKey,
    pub en: RegionKey,
    pub tw: RegionKey,
    #[serde(alias = "kr")]
    pub ko: RegionKey,
}

impl UserKeys {
    pub fn load() -> Self {
        crate::global::io::json::load("keys.json").unwrap_or_default()
    }

    pub fn save(&self) {
        crate::global::io::json::save("keys.json", self);
    }

    pub fn is_empty(&self) -> bool {
        ![Region::Ja, Region::En, Region::Tw, Region::Ko]
            .iter()
            .any(|&region| self.has_key_for(region))
    }

    pub fn has_key_for(&self, region: Region) -> bool {
        match region {
            Region::Ja => !self.ja.key.is_empty() && !self.ja.iv.is_empty(),
            Region::En => !self.en.key.is_empty() && !self.en.iv.is_empty(),
            Region::Tw => !self.tw.key.is_empty() && !self.tw.iv.is_empty(),
            Region::Ko => !self.ko.key.is_empty() && !self.ko.iv.is_empty(),
        }
    }

    pub fn as_tuples(&self) -> Vec<(String, String, Region)> {
        let mut vec = Vec::new();
        if self.has_key_for(Region::Ja) { vec.push((self.ja.key.clone(), self.ja.iv.clone(), Region::Ja)); }
        if self.has_key_for(Region::En) { vec.push((self.en.key.clone(), self.en.iv.clone(), Region::En)); }
        if self.has_key_for(Region::Tw) { vec.push((self.tw.key.clone(), self.tw.iv.clone(), Region::Tw)); }
        if self.has_key_for(Region::Ko) { vec.push((self.ko.key.clone(), self.ko.iv.clone(), Region::Ko)); }
        vec
    }

    pub fn validate(&self) -> [(bool, bool); 4] {
        let check = |val: &str, expected: &str| -> bool {
            if expected.is_empty() { return true; }
            let clean_val = val.trim();
            if clean_val.is_empty() { return false; }

            let hash = format!("{:x}", md5::compute(clean_val.as_bytes()));
            hash == expected
        };

        [
            (check(&self.ja.key, EXPECTED_HASHES[0].0), check(&self.ja.iv, EXPECTED_HASHES[0].1)),
            (check(&self.en.key, EXPECTED_HASHES[1].0), check(&self.en.iv, EXPECTED_HASHES[1].1)),
            (check(&self.tw.key, EXPECTED_HASHES[2].0), check(&self.tw.iv, EXPECTED_HASHES[2].1)),
            (check(&self.ko.key, EXPECTED_HASHES[3].0), check(&self.ko.iv, EXPECTED_HASHES[3].1)),
        ]
    }
}