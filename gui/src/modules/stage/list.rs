use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use iced::widget::{button, column, container, row, rule, scrollable, space, text};
use iced::{Element, Length, Theme};
use nyanko::chapter::Category;

use core::modules::stage::filter::{CompiledStageFilter, StageFilterState};
use core::modules::stage::{navigate, GlobalMapId, GlobalStageId, Map, StageDataState};

use super::category::CategoryExt;

const BTN_SPACING_Y: f32 = 6.0;
const CATEGORY_COLUMN_WIDTH: f32 = 180.0;
const COLUMN_WIDTH: f32 = 200.0;

#[derive(Debug, Clone)]
pub enum Message {
    ToggleFilter,
    SelectCategory(Category),
    SelectMap(GlobalMapId),
    SelectStage(GlobalStageId),
}

#[derive(Default)]
pub struct State {
    compiled_filter: Option<CompiledStageFilter>,
}

impl State {
    pub fn update(&mut self, message: Message, data: &mut StageDataState) {
        match message {
            Message::ToggleFilter => {}
            Message::SelectCategory(category) => {
                data.selected_category = Some(category);
                data.selected_map = None;
                data.selected_stage = None;
            }
            Message::SelectMap(map_id) => {
                data.selected_map = Some(map_id);
                data.selected_stage = None;
            }
            Message::SelectStage(stage_id) => {
                data.selected_stage = Some(stage_id);
            }
        }
    }

    pub fn refresh(&mut self, filter_state: &StageFilterState) {
        let mut hasher = DefaultHasher::new();
        filter_state.hash(&mut hasher);
        let current_hash = hasher.finish();

        if self.compiled_filter.as_ref().is_none_or(|cf| cf.source_hash != current_hash) {
            let mut compiled = filter_state.compile();
            compiled.source_hash = current_hash;
            self.compiled_filter = Some(compiled);
        }
    }

    pub fn view<'a>(&'a self, data: &'a StageDataState, filter_state: &StageFilterState) -> Element<'a, Message> {
        let categories = navigate::get_categories(&data.registry);

        if categories.is_empty() {
            return container(text("No Stages Found").style(|theme: &Theme| text::Style {
                color: Some(theme.palette().danger),
            }))
                .width(CATEGORY_COLUMN_WIDTH)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }

        let Some(compiled_filter) = self.compiled_filter.as_ref() else {
            return space().into();
        };

        let filter_btn_active = filter_state.is_active();
        let filter_btn = button(text("Filter Stages").size(13))
            .width(Length::Fill)
            .height(28)
            .on_press(Message::ToggleFilter)
            .style(move |theme: &Theme, _status| button::Style {
                background: Some(if filter_btn_active {
                    theme.palette().primary.into()
                } else {
                    theme.palette().background.into()
                }),
                text_color: theme.palette().text,
                ..Default::default()
            });

        let mut sidebar_row = row![].height(Length::Fill);

        let mut sorted_categories = categories;
        sorted_categories.sort_by_key(|category| category.sort_order());

        let mut cat_col = column![].spacing(BTN_SPACING_Y).width(CATEGORY_COLUMN_WIDTH);
        for category in sorted_categories {
            if !has_matching_stage_in_category(data, compiled_filter, &category) {
                continue;
            }

            let is_selected = data.selected_category.as_ref() == Some(&category);
            cat_col = cat_col.push(sidebar_button(
                category.display_name().to_string(),
                is_selected,
                Message::SelectCategory(category),
            ));
        }
        sidebar_row = sidebar_row.push(scrollable(cat_col).height(Length::Fill));

        if let Some(category) = &data.selected_category {
            let category_display_name = category.display_name();
            let mut map_col = column![].spacing(BTN_SPACING_Y).width(COLUMN_WIDTH);

            for map in navigate::get_maps(&data.registry, category) {
                let map_key = GlobalMapId { category: category.clone(), map: map.map_id };
                if !has_matching_stage_in_map(data, compiled_filter, category_display_name, &map_key, &map) {
                    continue;
                }

                let is_selected = data.selected_map.as_ref() == Some(&map_key);
                map_col = map_col.push(sidebar_button(map.name.clone(), is_selected, Message::SelectMap(map_key)));
            }
            sidebar_row = sidebar_row.push(scrollable(map_col).height(Length::Fill));
        }

        if let Some(map_id) = &data.selected_map {
            if let Some(map) = data.registry.maps.get(map_id) {
                let category_display_name = map_id.category.display_name();
                let mut stage_col = column![].spacing(BTN_SPACING_Y).width(COLUMN_WIDTH);

                for stage in navigate::get_stages(&data.registry, map_id) {
                    if !compiled_filter.matches(
                        category_display_name,
                        map,
                        &stage,
                        &data.enemy_name_registry,
                        &data.lock_skip_registry,
                        &data.scat_cpu_setting,
                        &data.item_buy_registry,
                        &data.item_name_registry,
                        &data.drop_chara_registry,
                        &data.unit_buy_registry,
                        &data.cat_name_registry,
                    ) {
                        continue;
                    }

                    let stage_key = GlobalStageId {
                        category: map_id.category.clone(),
                        map: map_id.map,
                        stage: stage.stage_id,
                    };
                    let is_selected = data.selected_stage.as_ref() == Some(&stage_key);
                    stage_col = stage_col.push(sidebar_button(stage.name.clone(), is_selected, Message::SelectStage(stage_key)));
                }
                sidebar_row = sidebar_row.push(scrollable(stage_col).height(Length::Fill));
            }
        }

        let mut sidebar_width = CATEGORY_COLUMN_WIDTH;
        if data.selected_category.is_some() {
            sidebar_width += COLUMN_WIDTH;
        }
        if data.selected_map.is_some() {
            sidebar_width += COLUMN_WIDTH;
        }

        column![
            container(filter_btn).padding(5),
            rule::horizontal(1),
            scrollable(sidebar_row)
                .direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::default()))
                .height(Length::Fill),
        ]
            .width(Length::Fixed(sidebar_width))
            .height(Length::Fill)
            .into()
    }
}

fn sidebar_button(label: String, is_selected: bool, msg: Message) -> Element<'static, Message> {
    button(text(label).size(13))
        .width(Length::Fill)
        .on_press(msg)
        .style(move |theme: &Theme, _status| button::Style {
            background: Some(if is_selected {
                theme.palette().primary.into()
            } else {
                theme.palette().background.into()
            }),
            text_color: theme.palette().text,
            ..Default::default()
        })
        .into()
}

fn has_matching_stage_in_map(
    data: &StageDataState,
    compiled_filter: &CompiledStageFilter,
    category_display_name: &str,
    map_key: &GlobalMapId,
    map: &Map,
) -> bool {
    if !compiled_filter.is_active() {
        return true;
    }

    navigate::get_stages(&data.registry, map_key).iter().any(|stage| {
        compiled_filter.matches(
            category_display_name,
            map,
            stage,
            &data.enemy_name_registry,
            &data.lock_skip_registry,
            &data.scat_cpu_setting,
            &data.item_buy_registry,
            &data.item_name_registry,
            &data.drop_chara_registry,
            &data.unit_buy_registry,
            &data.cat_name_registry,
        )
    })
}

fn has_matching_stage_in_category(data: &StageDataState, compiled_filter: &CompiledStageFilter, category: &Category) -> bool {
    if !compiled_filter.is_active() {
        return true;
    }

    navigate::get_maps(&data.registry, category).iter().any(|map| {
        let map_key = GlobalMapId { category: category.clone(), map: map.map_id };
        has_matching_stage_in_map(data, compiled_filter, category.display_name(), &map_key, map)
    })
}
