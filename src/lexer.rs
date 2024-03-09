#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum LexItem<'a> {
    At(&'a str),
    Paren(char),
    Word(&'a str),
    Whitespace(&'a str),
    NewLine,
}

impl<'a> LexItem<'a> {
    pub(crate) fn push_to(&self, acc: &mut String) {
        match self {
            LexItem::At(w) => acc.push_str(w),
            LexItem::Paren(w) => acc.push(*w),
            LexItem::Word(w) => acc.push_str(w),
            LexItem::Whitespace(w) => acc.push_str(w),
            LexItem::NewLine => acc.push('\n'),
        }
    }
}

pub(crate) fn lex(input: &str) -> Vec<LexItem<'_>> {
    let mut result = vec![];
    let mut start_index = 0;

    for (index, c) in input.char_indices() {
        match c {
            '@' => {
                result.push(LexItem::At(&input[index..index + c.len_utf8()]));
            }
            '\\' => match result.last_mut() {
                Some(LexItem::At(v)) if *v == "\\" => {
                    *v = &input[start_index..index + c.len_utf8()];
                }
                _ => {
                    start_index = index;
                    result.push(LexItem::At(&input[index..index + c.len_utf8()]));
                }
            },
            '{' | '}' => {
                result.push(LexItem::Paren(c));
            }
            ' ' | '\t' => match result.last_mut() {
                Some(LexItem::Whitespace(v)) => *v = &input[start_index..index + c.len_utf8()],
                _ => {
                    start_index = index;
                    result.push(LexItem::Whitespace(&input[index..index + c.len_utf8()]));
                }
            },
            '\n' => {
                result.push(LexItem::NewLine);
            }
            _ => match result.last_mut() {
                Some(LexItem::Word(v)) => *v = &input[start_index..index + c.len_utf8()],
                _ => {
                    start_index = index;
                    result.push(LexItem::Word(&input[index..index + c.len_utf8()]))
                }
            },
        }
    }

    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic_notation() {
        let result = lex("@name Memory Management");
        assert_eq!(
            result,
            vec![
                LexItem::At("@"),
                LexItem::Word("name"),
                LexItem::Whitespace(" "),
                LexItem::Word("Memory"),
                LexItem::Whitespace(" "),
                LexItem::Word("Management")
            ]
        );

        let result = lex("\\name Memory Management");
        assert_eq!(
            result,
            vec![
                LexItem::At("\\"),
                LexItem::Word("name"),
                LexItem::Whitespace(" "),
                LexItem::Word("Memory"),
                LexItem::Whitespace(" "),
                LexItem::Word("Management")
            ]
        );

        let result = lex("\\\\name Memory Management");
        assert_eq!(
            result,
            vec![
                LexItem::At("\\\\"),
                LexItem::Word("name"),
                LexItem::Whitespace(" "),
                LexItem::Word("Memory"),
                LexItem::Whitespace(" "),
                LexItem::Word("Management")
            ]
        );
    }

    #[test]
    fn basic_groups() {
        let result = lex("@{\n* @name Memory Management\n@}");
        assert_eq!(
            result,
            vec![
                LexItem::At("@"),
                LexItem::Paren('{'),
                LexItem::NewLine,
                LexItem::Word("*"),
                LexItem::Whitespace(" "),
                LexItem::At("@"),
                LexItem::Word("name"),
                LexItem::Whitespace(" "),
                LexItem::Word("Memory"),
                LexItem::Whitespace(" "),
                LexItem::Word("Management"),
                LexItem::NewLine,
                LexItem::At("@"),
                LexItem::Paren('}')
            ]
        );
    }
}
