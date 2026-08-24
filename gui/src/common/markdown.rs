use iced::widget::markdown;

const THIN_SPACE: char = '\u{2009}';

pub(crate) fn parse(source: &str) -> Vec<markdown::Item> {
    markdown::parse(&widen_inline_code(source)).collect()
}

fn widen_inline_code(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut in_fence = false;
    let mut in_code = false;

    for line in source.split_inclusive('\n') {
        if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
            in_fence = !in_fence;
            out.push_str(line);
            continue;
        }

        if in_fence {
            out.push_str(line);
            continue;
        }

        let chars: Vec<char> = line.chars().collect();

        for (index, &ch) in chars.iter().enumerate() {
            out.push(ch);

            if ch != '`' {
                continue;
            }

            in_code = !in_code;

            if !in_code && chars.get(index + 1).is_some_and(|next| trails_code(*next)) {
                out.push(THIN_SPACE);
            }
        }
    }

    out
}

fn trails_code(c: char) -> bool {
    matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}')
}

#[cfg(test)]
mod tests {
    use super::widen_inline_code;

    #[test]
    fn a_period_glued_to_a_code_span_gets_a_thin_space() {
        assert_eq!(widen_inline_code("the mark `!`."), "the mark `!`\u{2009}.");
    }

    #[test]
    fn a_code_span_followed_by_a_space_is_left_alone() {
        assert_eq!(widen_inline_code("`Raw` and `Resolved` modes"), "`Raw` and `Resolved` modes");
    }

    #[test]
    fn punctuation_inside_a_fenced_block_is_left_alone() {
        let source = "```\n`!`.\n```\n";
        assert_eq!(widen_inline_code(source), source);
    }

    #[test]
    fn multiple_spans_on_one_line_each_get_widened() {
        assert_eq!(widen_inline_code("`Add Mod`, and `Enable Mod`,"), "`Add Mod`\u{2009}, and `Enable Mod`\u{2009},");
    }
}
