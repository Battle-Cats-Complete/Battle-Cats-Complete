pub(crate) fn gatya_item_img(id: u32) -> String {
    format!("gatyaitemD_{:02}_f.png", id)
}

pub(crate) fn cat_form_img(id: u32, form: &str) -> String {
    format!("uni{:03}_{}00.png", id, form)
}

pub(crate) fn empty_cat_icon() -> String {
    "uni.png".to_string()
}

pub(crate) fn stage_name_targets(cat_prefix: &str) -> Vec<String> {
    let mut targets = vec![
        format!("StageName_{}.csv", cat_prefix),
        format!("StageName_R{}.csv", cat_prefix),
    ];

    if cat_prefix == "EC" {
        targets.push("StageName.csv".to_string());
    }

    match cat_prefix {
        "EC" => targets.push("StageName0.csv".to_string()),
        "W" => targets.push("StageName1.csv".to_string()),
        "Space" => targets.push("StageName2.csv".to_string()),
        _ => (),
    }

    targets
}