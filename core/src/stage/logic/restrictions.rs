use tracing::{debug, trace, warn};
use nyanko::common::tools::csv::{strip_html_tags, BreakHandling};
use nyanko::chapter::stage::{CharaGroupEntry, CharaGroupType};

use crate::global::context::GlobalContext;
use crate::stage::registry::Stage;

pub fn parse_restrictions(stage: &Stage, current_crown: i8, ctx: GlobalContext) -> Vec<String> {
    trace!(
        map_id = stage.map_id,
        stage_id = stage.stage_id,
        viewing_crown = current_crown,
        "parsing stage restrictions"
    );

    if stage.target_crowns != -1 && stage.target_crowns != current_crown {
        debug!(
            target = stage.target_crowns,
            current = current_crown,
            "Restriction does not apply to the currently viewed crown. Ignoring."
        );
        return Vec::new();
    }

    let effective_max_crowns = if stage.max_crowns == 0 { 1 } else { stage.max_crowns as i8 };
    if stage.target_crowns >= effective_max_crowns {
        debug!(
            target = stage.target_crowns,
            max = effective_max_crowns,
            "Restriction targets an unreachable crown. Ignoring junk data."
        );
        return Vec::new();
    }

    let mut restrictions = Vec::new();
    
    let effective_rarity_mask = if current_crown == 3 {
        6
    } else {
        stage.rarity_mask
    };

    if let Some(rarity_str) = parse_rarity_mask(effective_rarity_mask, ctx.clone()) {
        debug!(mask = effective_rarity_mask, "parsed rarity restriction");
        restrictions.push(rarity_str);
    }

    if stage.deploy_limit > 0 {
        trace!(limit = stage.deploy_limit, "adding deploy limit restriction");
        let raw_str = ctx.localizable.lookup("stage_restriction_limit_2").unwrap_or_default();
        let clean_str = strip_html_tags(&raw_str, BreakHandling::Space);

        if !clean_str.is_empty() {
            restrictions.push(clean_str.replace("%d", &stage.deploy_limit.to_string()));
        } else {
            restrictions.push(format!("stage_restriction_limit_2: {}", stage.deploy_limit));
        }
    }

    if stage.allowed_rows > 0 {
        trace!(rows = stage.allowed_rows, "adding row restriction");
        let raw_str = ctx.localizable.lookup("stage_restriction_limit_3").unwrap_or_default();
        let clean_str = strip_html_tags(&raw_str, BreakHandling::Space);

        if !clean_str.is_empty() {
            restrictions.push(clean_str.replace("%d", &stage.allowed_rows.to_string()));
        } else {
            restrictions.push(format!("stage_restriction_limit_3: {}", stage.allowed_rows));
        }
    }

    if stage.min_cost > 0 && stage.max_cost > 0 {
        trace!(min = stage.min_cost, max = stage.max_cost, "adding exact cost restriction");
        restrictions.push(format!("stage_restriction_cost_over: {} ~ stage_restriction_cost_under: {}", stage.min_cost, stage.max_cost));
    } else if stage.min_cost > 0 {
        trace!(min = stage.min_cost, "adding min cost restriction");
        let raw_cost = ctx.localizable.lookup("stage_restriction_cost_over").unwrap_or_default();
        let clean_cost = strip_html_tags(&raw_cost, BreakHandling::Space).replace("%d", &stage.min_cost.to_string());

        let raw_base = ctx.localizable.lookup("stage_restriction_limit_4").unwrap_or_default();
        let clean_base = strip_html_tags(&raw_base, BreakHandling::Space);

        if !clean_base.is_empty() && !clean_cost.is_empty() {
            restrictions.push(clean_base.replace("%@", &clean_cost));
        } else {
            restrictions.push(format!("stage_restriction_limit_4 + stage_restriction_cost_over: {}", stage.min_cost));
        }
    } else if stage.max_cost > 0 {
        trace!(max = stage.max_cost, "adding max cost restriction");
        let raw_cost = ctx.localizable.lookup("stage_restriction_cost_under").unwrap_or_default();
        let clean_cost = strip_html_tags(&raw_cost, BreakHandling::Space).replace("%d", &stage.max_cost.to_string());

        let raw_base = ctx.localizable.lookup("stage_restriction_limit_4").unwrap_or_default();
        let clean_base = strip_html_tags(&raw_base, BreakHandling::Space);

        if !clean_base.is_empty() && !clean_cost.is_empty() {
            restrictions.push(clean_base.replace("%@", &clean_cost));
        } else {
            restrictions.push(format!("stage_restriction_limit_4 + stage_restriction_cost_under: {}", stage.max_cost));
        }
    }

    if let Some(charagroup) = &stage.charagroup {
        if let Some(group_str) = parse_charagroup(charagroup, ctx.clone()) {
            debug!("parsed charagroup restriction");
            restrictions.push(group_str);
        } else {
            warn!(id = charagroup.id, "failed to parse charagroup string");
        }
    }

    trace!(
        count = restrictions.len(),
        "successfully parsed all stage restrictions"
    );

    restrictions
}

fn parse_rarity_mask(mask: u8, ctx: GlobalContext) -> Option<String> {
    if mask == 0 {
        return None;
    }

    let mut allowed = Vec::new();

    if mask & (1 << 0) != 0 { allowed.push("Normal"); }
    if mask & (1 << 1) != 0 { allowed.push("Special"); }
    if mask & (1 << 2) != 0 { allowed.push("Rare"); }
    if mask & (1 << 3) != 0 { allowed.push("Super Rare"); }
    if mask & (1 << 4) != 0 { allowed.push("Uber Rare"); }
    if mask & (1 << 5) != 0 { allowed.push("Legend Rare"); }

    let Some((last, first)) = allowed.split_last() else {
        warn!(mask, "rarity mask was non-zero but matched no known rarities");
        return None;
    };

    let rarity_list = if first.is_empty() {
        last.to_string()
    } else {
        format!("{} and {}", first.join(", "), last)
    };

    let raw_str = ctx.localizable.lookup("stage_restriction_limit_1").unwrap_or_default();
    let clean_str = strip_html_tags(&raw_str, BreakHandling::Space);

    if !clean_str.is_empty() {
        Some(clean_str.replace("%@", &rarity_list))
    } else {
        Some(format!("stage_restriction_limit_1: {}", rarity_list))
    }
}

fn parse_charagroup(group: &CharaGroupEntry, ctx: GlobalContext) -> Option<String> {
    let mode_str = match group.kind {
        CharaGroupType::OnlyUse => "Only",
        CharaGroupType::CannotUse => "Cannot use",
        _ => {
            warn!(id = group.id, "unknown charagroup type encountered");
            return None;
        }
    };

    let group_key = format!("stage_restriction_charagroup_{}", group.id);
    let raw_group_name = ctx.localizable.lookup(&group_key).unwrap_or_default();
    let mut group_name = strip_html_tags(&raw_group_name, BreakHandling::Space);

    if group_name.is_empty() {
        group_name = format!("{}: {} units", group_key, group.units.len());
    }

    let combined_val = format!("{} {}", mode_str, group_name);

    let raw_base = ctx.localizable.lookup("stage_restriction_limit_5").unwrap_or_default();
    let clean_base = strip_html_tags(&raw_base, BreakHandling::Space);

    if !clean_base.is_empty() {
        Some(clean_base.replace("%@", &combined_val))
    } else {
        Some(format!("stage_restriction_limit_5: {}", combined_val))
    }
}