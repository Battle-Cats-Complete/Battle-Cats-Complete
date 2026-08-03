use std::collections::HashMap;

use core::modules::cat::scanner::CatEntry;

pub struct Ctx<'a> {
    pub cat: Option<&'a CatEntry>,
    pub form: usize,
    pub talent_levels: Option<&'a HashMap<u8, u8>>,
    pub bump_enabled: bool,
}

#[derive(Default)]
pub struct State {
    is_active: bool,
    saved_level: Option<(i32, String)>,
}

impl State {
    pub fn sync(&mut self, ctx: Ctx<'_>, current_level: &mut i32, level_input: &mut String) {
        let Some(is_ultra) = ultra_state(&ctx) else {
            return;
        };

        if !ctx.bump_enabled {
            self.is_active = is_ultra;
            self.saved_level = None;
            return;
        }

        if !self.is_active && is_ultra {
            self.saved_level = Some((*current_level, level_input.clone()));
            if *current_level < 60 {
                *current_level = 60;
                *level_input = "60".to_string();
            }
        } else if self.is_active && !is_ultra
            && let Some((saved_level, saved_input)) = self.saved_level.take() {
            let expected_level = if saved_level < 60 { 60 } else { saved_level };
            if *current_level == expected_level {
                *current_level = saved_level;
                *level_input = saved_input;
            }
        }
        self.is_active = is_ultra;
    }
}

fn ultra_state(ctx: &Ctx<'_>) -> Option<bool> {
    let cat = ctx.cat?;

    let mut ultra = ctx.form == 3;
    if ctx.form >= 2 && !ultra
        && let Some(levels) = ctx.talent_levels {
        if let Some(talent_data) = &cat.talent_data {
            ultra = talent_data.groups.iter().enumerate().any(|(idx, group)| {
                group.limit == 1
                    && levels.get(&(idx as u8)).is_some_and(|&lvl| lvl > 0)
            });
        } else {
            ultra = levels.iter().any(|(&idx, &lvl)| idx >= 5 && lvl > 0);
        }
    }

    Some(ultra)
}
