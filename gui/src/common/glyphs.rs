const WIDE: [(u32, u32); 12] = [
    (0x1100, 0x115f),
    (0x2e80, 0x303e),
    (0x3041, 0x33ff),
    (0x3400, 0x4dbf),
    (0x4e00, 0x9fff),
    (0xa000, 0xa4cf),
    (0xac00, 0xd7a3),
    (0xf900, 0xfaff),
    (0xfe30, 0xfe6f),
    (0xff00, 0xff60),
    (0xffe0, 0xffe6),
    (0x20000, 0x3fffd),
];

pub(crate) fn wide(glyph: char) -> bool {
    let code = glyph as u32;

    WIDE.iter().any(|(first, last)| code >= *first && code <= *last)
}

pub(crate) fn columns(text: &str) -> f32 {
    text.chars().map(|glyph| if wide(glyph) { 2.0 } else { 1.0 }).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kana_and_kanji_take_two_columns() {
        // Part names in the game data are Japanese, and measuring them as one
        // column each is what clipped the labels.
        assert_eq!(columns("リンゴ"), 6.0);
        assert_eq!(columns("土"), 2.0);
        assert_eq!(columns("abc"), 3.0);
    }

    #[test]
    fn a_mixed_label_counts_each_half_separately() {
        assert_eq!(columns("067土1.png"), 10.0);
    }

    #[test]
    fn punctuation_around_the_ranges_stays_narrow() {
        assert!(!wide('·'));
        assert!(!wide('#'));
        assert!(wide('\u{ff21}'));
    }
}
