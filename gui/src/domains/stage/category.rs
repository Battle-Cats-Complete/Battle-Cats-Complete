use nyanko::chapter::Category;

pub(crate) trait CategoryExt {
    fn display_name(&self) -> &str;
    fn sort_order(&self) -> u32;
}

impl CategoryExt for Category {
    fn display_name(&self) -> &str {
        match self {
            Self::StoriesOfLegend      => "Stories of Legend",
            Self::EventStages          => "Event Stages",
            Self::CollabStages         => "Collab Stages",
            Self::EmpireOfCats         => "Empire of Cats",
            Self::IntoTheFuture        => "Into the Future",
            Self::CatsOfTheCosmos      => "Cats of the Cosmos",
            Self::ExtraStages          => "Extra Stages",
            Self::DojoHallOfInitiates  => "Dojo Hall of Initiates",
            Self::TowersAndCitadels    => "Towers & Citadels",
            Self::DojoRankingEvents    => "Dojo Ranking Events",
            Self::ChallengeBattle      => "Challenge Battle",
            Self::UncannyLegends       => "Uncanny Legends",
            Self::CataminStages        => "Catamin Stages",
            Self::LegendQuest          => "Legend Quest",
            Self::ZombieOutbreaks      => "Zombie Outbreaks",
            Self::GauntletStages       => "Gauntlet Stages",
            Self::EnigmaStages         => "Enigma Stages",
            Self::CollabGauntletStages => "Collab Gauntlet Stages",
            Self::AkuRealms            => "Aku Realms",
            Self::BehemothCulling      => "Behemoth Culling",
            Self::Labyrinth            => "Labyrinth",
            Self::ZeroLegends          => "Zero Legends",
            Self::OtherworldColosseum  => "Otherworld Colosseum",
            Self::CatclawChampionships => "Catclaw Championships",
            Self::Unknown(prefix)      => {
                if prefix == "PT" {
                    "Legacy Princess Punt"
                } else {
                    prefix.as_str()
                }
            }
        }
    }

    fn sort_order(&self) -> u32 {
        match self {
            Self::EmpireOfCats         => 0,
            Self::IntoTheFuture        => 1,
            Self::CatsOfTheCosmos      => 2,
            Self::ZombieOutbreaks      => 3,
            Self::AkuRealms            => 4,

            Self::StoriesOfLegend      => 10,
            Self::UncannyLegends       => 11,
            Self::ZeroLegends          => 12,


            Self::EventStages          => 20,
            Self::ExtraStages          => 21,
            Self::GauntletStages       => 22,
            Self::CataminStages        => 24,
            Self::ChallengeBattle      => 25,

            Self::TowersAndCitadels    => 31,
            Self::OtherworldColosseum  => 32,
            Self::Labyrinth            => 33,
            Self::LegendQuest          => 34,

            Self::BehemothCulling      => 41,
            Self::EnigmaStages         => 42,

            Self::DojoHallOfInitiates  => 51,
            Self::DojoRankingEvents    => 52,
            Self::CatclawChampionships => 53,

            Self::CollabStages         => 61,
            Self::CollabGauntletStages => 62,
            Self::Unknown(prefix)      => {
                if prefix == "PT" {
                    63
                } else {
                    99
                }
            }
        }
    }
}