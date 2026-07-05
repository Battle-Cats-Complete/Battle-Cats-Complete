use nyanko::common::utils::csv::{strip_html_tags, BreakHandling};
use tracing::{debug, instrument, warn};

use crate::global::context::GlobalContext;
use crate::stage::data::specialrulesmap::{RuleType, SpecialRule};
use crate::stage::data::specialrulesmapoption::SpecialRuleOption;

#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ProcessedRule {
    pub title: String,
    pub description: String,
    pub invalid_combos: Vec<u32>,
}

#[instrument(skip(rule, options, ctx))]
pub fn parse(
    rule: &SpecialRule,
    options: &std::collections::HashMap<u8, SpecialRuleOption>,
    ctx: &GlobalContext,
) -> ProcessedRule {
    debug!(label = %rule.name_label, "parsing special rule");

    let raw_title = ctx.localizable.lookup(&rule.name_label).unwrap_or_default();

    let exp_key = rule.name_label.replace("Name", "Explanation");
    let raw_desc = ctx.localizable.lookup(&exp_key).unwrap_or_default();

    let mut title = strip_html_tags(&raw_title, BreakHandling::Space);
    let mut description = strip_html_tags(&raw_desc, BreakHandling::Space);

    if title.is_empty() {
        warn!(key = %rule.name_label, "missing localization for special rule title");
        title = rule.name_label.clone();
    }

    if description.is_empty() {
        warn!(key = %exp_key, "falling back to raw enum parsing");
        description = format!("{}:\n{}", exp_key, fallback_description(rule));
    }

    let mut invalid_combos = Vec::new();

    // 3. Extract Invalid Combos
    for target_rule in &rule.rules {
        let rule_id = match target_rule {
            RuleType::TrustFund(_) => 0,
            RuleType::CooldownEquality(_) => 1,
            RuleType::RarityLimit(_) => 3,
            RuleType::CheapLabor(_) => 4,
            RuleType::RestrictPrice(_) => 5,
            RuleType::RestrictCd(_) => 6,
            RuleType::DeployLimit(_) => 7,
            RuleType::AwesomeCatSpawn(_) => 8,
            RuleType::AwesomeCatCannon(_) => 9,
            RuleType::AwesomeUnitSpeed(_) => 10,
            RuleType::Unknown(id, _) => *id,
        };

        if let Some(opt) = options.get(&rule_id) {
            invalid_combos.extend(&opt.invalid_combo_ids);
        }
    }

    invalid_combos.sort_unstable();
    invalid_combos.dedup();

    ProcessedRule {
        title,
        description,
        invalid_combos,
    }
}

fn fallback_description(rule: &SpecialRule) -> String {
    let mut description = String::new();

    for target_rule in &rule.rules {
        let formatted_rule = match target_rule {
            RuleType::TrustFund(params) => format!("Trust Fund (Params: {:?})", params),
            RuleType::CooldownEquality(params) => format!("Cooldown Equality (Params: {:?})", params),
            RuleType::RarityLimit(params) => format!("Rarity Limit (Params: {:?})", params),
            RuleType::CheapLabor(params) => format!("Cheap Labor (Params: {:?})", params),
            RuleType::RestrictPrice(params) => format!("Restrict Price (Params: {:?})", params),
            RuleType::RestrictCd(params) => format!("Restrict CD (Params: {:?})", params),
            RuleType::DeployLimit(params) => format!("Deploy Limit (Params: {:?})", params),
            RuleType::AwesomeCatSpawn(params) => format!("Awesome Cat Spawn (Params: {:?})", params),
            RuleType::AwesomeCatCannon(params) => format!("Awesome Cat Cannon (Params: {:?})", params),
            RuleType::AwesomeUnitSpeed(params) => format!("Awesome Unit Speed (Params: {:?})", params),
            RuleType::Unknown(id, params) => format!("Unknown Rule {} (Params: {:?})", id, params),
        };
        description.push_str(&formatted_rule);
        description.push('\n');
    }

    description.trim().to_string()
}