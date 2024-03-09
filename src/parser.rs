use crate::lexer::{lex, LexItem};

const OPEN_PAREN: char = '{';
const CLOSED_PAREN: char = '}';

#[derive(Debug, Clone)]
pub enum ParseError {
    UnexpectedEndOfInput,
    UnexpectedInput {
        found: String,
        expected: Vec<String>,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum GrammarItem<'a> {
    Notation {
        meta: Vec<&'a str>,
        params: Vec<&'a str>,
        tag: &'a str,
    },
    Text(String),
    GroupStart,
    GroupEnd,
}

enum ParamParser {
    None,
    Whitespace,
    Paren,
}

pub(crate) fn parse(input: &str) -> Result<Vec<GrammarItem<'_>>, ParseError> {
    let lexed = lex(input);
    parse_items(lexed)
}

fn parse_items(input: Vec<LexItem>) -> Result<Vec<GrammarItem<'_>>, ParseError> {
    let mut grammar_items = vec![];
    let mut param_iter_skip_count = 0;

    for (index, current) in input.iter().enumerate() {
        let rest = &input[index..];
        let next = rest.get(1);

        if param_iter_skip_count > 0 {
            param_iter_skip_count -= 1;
            continue;
        }

        // Do not do any formatting inside of code blocks
        let ends_code = matches!(current, LexItem::At(_))
            && matches!(next, Some(LexItem::Word(v)) if *v == "endcode");
        if !ends_code {
            match &mut grammar_items[..] {
                [.., GrammarItem::Notation { tag, .. }] if *tag == "code" => {
                    let mut text = String::new();
                    current.push_to(&mut text);

                    grammar_items.push(GrammarItem::Text(text));
                    continue;
                }
                [.., GrammarItem::Notation { tag, .. }, GrammarItem::Text(text)]
                    if *tag == "code" =>
                {
                    current.push_to(text);
                    continue;
                }
                _ => {}
            }
        }

        match current {
            LexItem::At(_) => {
                if let Some(next) = next {
                    match next {
                        LexItem::Paren(v) => match *v {
                            OPEN_PAREN => grammar_items.push(GrammarItem::GroupStart),
                            CLOSED_PAREN => grammar_items.push(GrammarItem::GroupEnd),
                            _ => {
                                return Err(ParseError::UnexpectedInput {
                                    found: v.to_string(),
                                    expected: vec![OPEN_PAREN.into(), CLOSED_PAREN.into()],
                                })
                            }
                        },
                        LexItem::Word(v) => {
                            let mut meta = vec![];
                            let tag;

                            let param_parser;

                            if v.starts_with("param") {
                                match v.split_once('[') {
                                    Some((_, "in]")) => meta.push("in"),
                                    Some((_, "out]")) => meta.push("out"),
                                    Some((_, "in,out]")) | Some((_, "out,in]")) => {
                                        meta.push("in");
                                        meta.push("out");
                                    }
                                    None => {}
                                    Some((_, v)) => {
                                        return Err(ParseError::UnexpectedInput {
                                            found: v.to_string(),
                                            expected: vec!["in]".into(), "out]".into()],
                                        })
                                    }
                                }

                                tag = "param";
                                param_parser = ParamParser::Whitespace;
                            } else {
                                tag = v;
                                param_parser = match *v {
                                    "a" | "b" | "c" | "p" | "emoji" | "e" | "em" | "def"
                                    | "class" | "category" | "concept" | "enum" | "example"
                                    | "extends" | "file" | "sa" | "see" | "retval"
                                    | "exception" | "throw" | "throws" => ParamParser::Whitespace,
                                    "code" => ParamParser::Paren,
                                    _ => ParamParser::None,
                                };
                            }

                            let param = match param_parser {
                                ParamParser::None => None,
                                ParamParser::Whitespace => rest
                                    .iter()
                                    .enumerate()
                                    .skip(2)
                                    .find(|(_, next)| !matches!(next, LexItem::Whitespace(_)))
                                    .and_then(|(skip, next)| match next {
                                        LexItem::Word(word) => Some((skip, *word)),
                                        _ => None,
                                    }),
                                ParamParser::Paren => match &rest {
                                    [_, _, LexItem::Paren('{'), LexItem::Word(word), LexItem::Paren('}'), ..] => {
                                        Some((4, *word))
                                    }
                                    _ => None,
                                },
                            };

                            let params = if let Some((skip, word)) = param {
                                param_iter_skip_count = skip;
                                vec![word]
                            } else {
                                param_iter_skip_count = 1;
                                vec![]
                            };

                            grammar_items.push(GrammarItem::Notation { meta, params, tag });

                            if tag == "endcode" {
                                grammar_items.push(GrammarItem::Text("".into()));
                            }
                        }
                        _ => {}
                    }
                }
            }
            LexItem::Word(v) => match grammar_items.last_mut() {
                Some(GrammarItem::Text(text)) => text.push_str(v),
                _ => grammar_items.push(GrammarItem::Text(v.to_string())),
            },
            LexItem::Whitespace(_) => match grammar_items.last_mut() {
                Some(GrammarItem::Text(text)) => text.push(' '),
                Some(GrammarItem::Notation { params, .. }) if !params.is_empty() => {
                    grammar_items.push(GrammarItem::Text(" ".into()))
                }
                None => grammar_items.push(GrammarItem::Text(" ".into())),
                _ => grammar_items.push(GrammarItem::Text("".into())),
            },
            LexItem::NewLine => {
                if let Some(GrammarItem::Text(text)) = grammar_items.last_mut() {
                    text.push('\n');
                }
            }
            LexItem::Paren(v) => {
                if let Some(GrammarItem::Text(text)) = grammar_items.last_mut() {
                    text.push(*v);
                }
            }
        }
    }

    Ok(grammar_items)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn simple_notation() {
        let result = parse("@name Memory Management").unwrap();
        assert_eq!(
            result,
            vec![
                GrammarItem::Notation {
                    meta: vec![],
                    params: vec![],
                    tag: "name",
                },
                GrammarItem::Text("Memory Management".into())
            ]
        );
    }

    #[test]
    pub fn paren_in_notation() {
        let result = parse("@note hoge_t = {a, b, c}").unwrap();
        assert_eq!(
            result,
            vec![
                GrammarItem::Notation {
                    meta: vec![],
                    params: vec![],
                    tag: "note",
                },
                GrammarItem::Text("hoge_t = {a, b, c}".into())
            ]
        );
    }

    #[test]
    pub fn param() {
        let result =
            parse("@param[in] random This is, without a doubt, a random argument.").unwrap();
        assert_eq!(
            result,
            vec![
                GrammarItem::Notation {
                    meta: vec!["in"],
                    params: vec!["random"],
                    tag: "param",
                },
                GrammarItem::Text(" This is, without a doubt, a random argument.".into())
            ]
        );
    }

    #[test]
    pub fn param_tabs() {
        let result =
            parse("@param[in]\trandom\t\t\tThis is, without a doubt, a random argument.").unwrap();
        assert_eq!(
            result,
            vec![
                GrammarItem::Notation {
                    meta: vec!["in"],
                    params: vec!["random"],
                    tag: "param",
                },
                GrammarItem::Text(" This is, without a doubt, a random argument.".into())
            ]
        );
    }

    #[test]
    pub fn groups() {
        let result = parse("@{\n* @name Memory Management\n@}").unwrap();
        assert_eq!(
            result,
            vec![
                GrammarItem::GroupStart,
                GrammarItem::Text("* ".into()),
                GrammarItem::Notation {
                    meta: vec![],
                    params: vec![],
                    tag: "name",
                },
                GrammarItem::Text("Memory Management\n".into()),
                GrammarItem::GroupEnd
            ]
        );
    }

    #[test]
    pub fn trims_param_texts() {
        let result = parse(
            "@param[in]           var                                         Example description",
        )
        .unwrap();
        assert_eq!(
            result,
            vec![
                GrammarItem::Notation {
                    meta: vec!["in"],
                    params: vec!["var"],
                    tag: "param",
                },
                GrammarItem::Text(" Example description".into())
            ]
        )
    }

    #[test]
    pub fn code() {
        let result = parse("@code\nfn main() {}\n@endcode").unwrap();

        assert_eq!(
            result,
            vec![
                GrammarItem::Notation {
                    meta: vec![],
                    params: vec![],
                    tag: "code",
                },
                GrammarItem::Text("\nfn main() {}\n".into()),
                GrammarItem::Notation {
                    meta: vec![],
                    params: vec![],
                    tag: "endcode",
                },
                GrammarItem::Text("".into())
            ]
        )
    }

    #[test]
    pub fn code_with_param() {
        let result = parse("@code{.py}\nfn main() {}\n@endcode").unwrap();

        assert_eq!(
            result,
            vec![
                GrammarItem::Notation {
                    meta: vec![],
                    params: vec![".py"],
                    tag: "code",
                },
                GrammarItem::Text("\nfn main() {}\n".into()),
                GrammarItem::Notation {
                    meta: vec![],
                    params: vec![],
                    tag: "endcode",
                },
                GrammarItem::Text("".into())
            ]
        )
    }

    #[test]
    pub fn code_with_args() {
        let result = parse("@code\nfn main() {}\n@endcode\n\n@param[in] a - a").unwrap();

        assert_eq!(
            result,
            vec![
                GrammarItem::Notation {
                    meta: vec![],
                    params: vec![],
                    tag: "code",
                },
                GrammarItem::Text("\nfn main() {}\n".into()),
                GrammarItem::Notation {
                    meta: vec![],
                    params: vec![],
                    tag: "endcode",
                },
                GrammarItem::Text("\n\n".into()),
                GrammarItem::Notation {
                    meta: vec!["in"],
                    params: vec!["a"],
                    tag: "param"
                },
                GrammarItem::Text(" - a".into())
            ]
        )
    }
}
