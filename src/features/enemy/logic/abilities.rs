use crate::global::img015;
use crate::features::settings::logic::Settings;
use crate::features::enemy::data::t_unit::EnemyRaw;
use crate::features::enemy::registry::{self, DisplayGroup};

#[derive(Clone, PartialEq, Default, Debug)]
pub enum EnemyCustomIcon {
    #[default] None,
    Multihit,
    Kamikaze,
}

#[derive(Clone, Debug)]
pub struct EnemyAbilityItem {
    pub icon_id: usize,
    pub text: String,
    pub custom_icon: EnemyCustomIcon,
    pub border_id: Option<usize>,
}

pub fn collect_ability_data(
    stats: &EnemyRaw,
    settings: &Settings,
    magnification: i32,
) -> (Vec<EnemyAbilityItem>, Vec<EnemyAbilityItem>, Vec<EnemyAbilityItem>, Vec<EnemyAbilityItem>, Vec<EnemyAbilityItem>, Vec<EnemyAbilityItem>) {
    
    let mut group_trait = Vec::new();
    let mut group_headline_1 = Vec::new();
    let mut group_headline_2 = Vec::new();
    let mut group_body_1 = Vec::new();
    let mut group_body_2 = Vec::new();
    let mut group_footer = Vec::new();

    if stats.attack_2 > 0 {
        let damage_hit_1 = stats.attack_1;
        let damage_hit_2 = stats.attack_2;
        let damage_hit_3 = stats.attack_3;
        
        let ability_flag_1 = if stats.attack_1_abilities > 0 { "True" } else { "False" };
        let ability_flag_2 = if stats.attack_2_abilities > 0 { "True" } else { "False" };
        let ability_flag_3 = if stats.attack_3 > 0 { if stats.attack_3_abilities > 0 { " / True" } else { " / False" } } else { "" };
        
        let damage_string = if stats.attack_3 > 0 { 
            format!("{} / {} / {}", damage_hit_1, damage_hit_2, damage_hit_3) 
        } else { 
            format!("{} / {}", damage_hit_1, damage_hit_2) 
        };
        let multihit_description = format!("Damage split {}\nAbility split {} / {}{}", damage_string, ability_flag_1, ability_flag_2, ability_flag_3);
        let custom_icon = if settings.game_language == "--" { EnemyCustomIcon::None } else { EnemyCustomIcon::Multihit };

        group_body_1.push(EnemyAbilityItem { icon_id: img015::ICON_MULTIHIT, text: multihit_description, custom_icon, border_id: None });
    }

    range_logic(stats, &mut group_body_1);

    for def in registry::ENEMY_ABILITY_REGISTRY {
        let val = (def.getter)(stats);
        if val > 0 || val == -1 {
            let dur = if let Some(d_get) = def.duration_getter { d_get(stats) } else { 0 };
            let text = (def.formatter)(val, stats, dur, magnification);

            let mut final_icon = def.icon_id;
            if def.name == "Wave Attack" && stats.mini_wave > 0 { final_icon = img015::ICON_MINI_WAVE; }
            else if def.name == "Surge Attack" && stats.mini_surge > 0 { final_icon = img015::ICON_MINI_SURGE; }

            let item = EnemyAbilityItem { icon_id: final_icon, text, custom_icon: EnemyCustomIcon::None, border_id: None };

            match def.group {
                DisplayGroup::Type => group_trait.push(item),
                DisplayGroup::Headline1 => group_headline_1.push(item),
                DisplayGroup::Headline2 => group_headline_2.push(item),
                DisplayGroup::Body1 => group_body_1.push(item),
                DisplayGroup::Body2 => group_body_2.push(item),
                DisplayGroup::Footer => group_footer.push(item),
            }
        }
    }

    if stats.kamikaze > 0 {
         let item = EnemyAbilityItem { icon_id: img015::ICON_KAMIKAZE, text: "Unit disappears after a single attack".into(), custom_icon: EnemyCustomIcon::Kamikaze, border_id: None };
         group_headline_2.push(item);
    }

    (group_trait, group_headline_1, group_headline_2, group_body_1, group_body_2, group_footer)
}

fn range_logic(stats: &EnemyRaw, group_body_1: &mut Vec<EnemyAbilityItem>) {
    let enemy_base_range = {
        let start_range = stats.long_distance_anchor_1;
        let end_range = stats.long_distance_anchor_1 + stats.long_distance_span_1;
        let (min_reach, max_reach) = if start_range < end_range { (start_range, end_range) } else { (end_range, start_range) };
        if min_reach > 0 { min_reach } else { max_reach }
    };

    let mut is_omni_strike = false;
    let mut range_strings = Vec::new();
    let range_checks = [
        (stats.long_distance_anchor_1, stats.long_distance_span_1, 1),
        (stats.long_distance_2_anchor, stats.long_distance_2_span, stats.long_distance_2_flag),
        (stats.long_distance_3_anchor, stats.long_distance_3_span, stats.long_distance_3_flag),
    ];
    
    for (anchor, span, flag) in range_checks {
        if flag > 0 && span != 0 {
            let start = anchor;
            let end = anchor + span;
            let (min, max) = if start < end { (start, end) } else { (end, start) };
            if min <= 0 { is_omni_strike = true; }
            range_strings.push(format!("{}~{}", min, max));
        }
    }

    if range_strings.len() > 1 {
        let first = &range_strings[0];
        if range_strings.iter().all(|s| s == first) {
            range_strings.truncate(1);
        }
    }

    if !range_strings.is_empty() {
        let label_prefix = if range_strings.len() > 1 { "Range split" } else { "Effective Range" };
        let range_description = format!(
            "{} {}\nStands at {} Range relative to Cat Base", 
            label_prefix,
            range_strings.join(" / "), 
            enemy_base_range
        );
        let icon = if is_omni_strike { img015::ICON_OMNI_STRIKE } else { img015::ICON_LONG_DISTANCE };
        group_body_1.push(EnemyAbilityItem { icon_id: icon, text: range_description, custom_icon: EnemyCustomIcon::None, border_id: None });
    }
}