use image::{ImageFormat, Rgba, RgbaImage};

pub const SEED_SUFFIX: &str = "00";

const SHEET_SPAN: u32 = 128;
const BOX_SPAN: u32 = 64;
const BOX_INK: Rgba<u8> = Rgba([90, 160, 235, 255]);
const BOX_EDGE: Rgba<u8> = Rgba([235, 245, 255, 255]);
const EDGE_WIDTH: u32 = 4;

pub(super) fn sheet() -> Vec<u8> {
    let mut art = RgbaImage::new(SHEET_SPAN, SHEET_SPAN);

    for y in 0..BOX_SPAN {
        for x in 0..BOX_SPAN {
            let rim = x < EDGE_WIDTH
                || y < EDGE_WIDTH
                || x >= BOX_SPAN - EDGE_WIDTH
                || y >= BOX_SPAN - EDGE_WIDTH;

            art.put_pixel(x, y, if rim { BOX_EDGE } else { BOX_INK });
        }
    }

    let mut encoded = std::io::Cursor::new(Vec::new());

    if let Err(err) = art.write_to(&mut encoded, ImageFormat::Png) {
        tracing::warn!("Studio could not encode the seed atlas: {}", err);
    }

    encoded.into_inner()
}

pub(super) fn cuts(name: &str) -> String {
    format!("[imgcut]\n1\n{}.png\n1\n0,0,{},{},box\n", name, BOX_SPAN, BOX_SPAN)
}

pub(super) fn model() -> String {
    let half = BOX_SPAN / 2;
    let (align_x, align_y) = (half, BOX_SPAN);

    format!(
        "[modelanim:model]\n1\n1\n-1,0,0,0,0,0,{half},{half},1000,1000,0,1000,0,box\n1000,3600,1000\n2\n0,0,{align_x},{align_y},0,0,combat\n0,0,{align_x},{align_y},0,0,gacha\n"
    )
}

pub(super) fn track() -> String {
    "[modelanim:animation]\n1\n1\n0,11,-1,0,0,spin\n2\n0,0,0,0\n60,3600,0,0\n".to_owned()
}

#[cfg(test)]
mod tests {
    use nyanko::graphics::rig::Model;

    use super::*;
    use crate::systems::animation::authoring::{Imgcut, Maanim};

    #[test]
    fn every_seeded_document_parses_back() {
        // The seed is what "New Set" writes, so a malformed one is a broken button.
        let parsed = Imgcut::parse(cuts("Test").as_bytes()).expect("the cut list parses");
        assert_eq!(parsed.count(), 1);
        assert_eq!(parsed.sheet(), "Test.png");

        let parsed = Model::parse(model().as_bytes()).expect("the model parses");
        assert_eq!(parsed.parts.len(), 1);
        assert_eq!(parsed.alignment.len(), 2);

        let parsed = Maanim::parse(track().as_bytes()).expect("the animation parses");
        assert_eq!(parsed.tracks().len(), 1);
    }

    #[test]
    fn the_seeded_entity_stands_on_the_ground_line() {
        // A box hanging below the origin is an authoring mistake, so the seed must not
        // ship one. Both numbers come straight from `animate::shift` and `engine::deploy`.
        let model = Model::parse(model().as_bytes()).expect("the model parses");
        let root = model.parts.first().expect("the root part");
        let align = model.alignment.first().expect("the combat row");

        let unit = model.scale_unit as f32;
        let shift = |offset: i32, pivot: i32, scale: i32| {
            (-(offset as f32) + pivot as f32) * (scale as f32 / unit)
        };

        let top = -(root.pivot_y as f32) + shift(align.y, root.pivot_y, root.scale_y);
        let left = -(root.pivot_x as f32) + shift(align.x, root.pivot_x, root.scale_x);

        assert_eq!(top + BOX_SPAN as f32, 0.0, "its feet rest on the origin");
        assert_eq!(left, -(BOX_SPAN as f32) / 2.0, "and it is centred on it");
    }

    #[test]
    fn the_seed_atlas_decodes_at_the_declared_span() {
        let decoded = image::load_from_memory(&sheet()).expect("the atlas decodes");

        assert_eq!((decoded.width(), decoded.height()), (SHEET_SPAN, SHEET_SPAN));
    }
}
