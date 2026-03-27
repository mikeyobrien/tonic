#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Doc {
    Nil,
    Concat(Box<Doc>, Box<Doc>),
    Nest(i32, Box<Doc>),
    Text(String),
    Line,
    SoftLine,
    Group(Box<Doc>),
    FlexBreak(Box<Doc>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Flat,
    Broken,
}

#[derive(Debug, Clone, Copy)]
struct Command<'a> {
    indent: usize,
    mode: Mode,
    doc: &'a Doc,
}

pub(crate) fn format(doc: &Doc, max_width: usize) -> String {
    let mut output = String::new();
    let mut column = 0usize;
    let mut stack = vec![Command {
        indent: 0,
        mode: Mode::Broken,
        doc,
    }];

    while let Some(command) = stack.pop() {
        match command.doc {
            Doc::Nil => {}
            Doc::Concat(left, right) => {
                stack.push(Command {
                    indent: command.indent,
                    mode: command.mode,
                    doc: right,
                });
                stack.push(Command {
                    indent: command.indent,
                    mode: command.mode,
                    doc: left,
                });
            }
            Doc::Nest(delta, inner) => {
                stack.push(Command {
                    indent: adjust_indent(command.indent, *delta),
                    mode: command.mode,
                    doc: inner,
                });
            }
            Doc::Text(text) => {
                output.push_str(text);
                column += display_width(text);
            }
            Doc::Line => match command.mode {
                Mode::Flat => {
                    output.push(' ');
                    column += 1;
                }
                Mode::Broken => {
                    output.push('\n');
                    for _ in 0..command.indent {
                        output.push(' ');
                    }
                    column = command.indent;
                }
            },
            Doc::SoftLine => match command.mode {
                Mode::Flat => {}
                Mode::Broken => {
                    output.push('\n');
                    for _ in 0..command.indent {
                        output.push(' ');
                    }
                    column = command.indent;
                }
            },
            Doc::Group(inner) => {
                let mode = if fits(
                    max_width as isize - column as isize,
                    &stack,
                    Command {
                        indent: command.indent,
                        mode: Mode::Flat,
                        doc: inner,
                    },
                ) {
                    Mode::Flat
                } else {
                    Mode::Broken
                };

                stack.push(Command {
                    indent: command.indent,
                    mode,
                    doc: inner,
                });
            }
            Doc::FlexBreak(inner) => {
                let mode = match command.mode {
                    Mode::Flat => Mode::Flat,
                    Mode::Broken => {
                        if fits(
                            max_width as isize - column as isize,
                            &stack,
                            Command {
                                indent: command.indent,
                                mode: Mode::Flat,
                                doc: inner,
                            },
                        ) {
                            Mode::Flat
                        } else {
                            Mode::Broken
                        }
                    }
                };

                stack.push(Command {
                    indent: command.indent,
                    mode,
                    doc: inner,
                });
            }
        }
    }

    output
}

fn fits(remaining: isize, stack: &[Command<'_>], next: Command<'_>) -> bool {
    if remaining < 0 {
        return false;
    }

    let mut remaining = remaining;
    let mut probe = stack.to_vec();
    probe.push(next);

    while remaining >= 0 {
        let Some(command) = probe.pop() else {
            return true;
        };

        match command.doc {
            Doc::Nil => {}
            Doc::Concat(left, right) => {
                probe.push(Command {
                    indent: command.indent,
                    mode: command.mode,
                    doc: right,
                });
                probe.push(Command {
                    indent: command.indent,
                    mode: command.mode,
                    doc: left,
                });
            }
            Doc::Nest(delta, inner) => {
                probe.push(Command {
                    indent: adjust_indent(command.indent, *delta),
                    mode: command.mode,
                    doc: inner,
                });
            }
            Doc::Text(text) => {
                remaining -= display_width(text) as isize;
            }
            Doc::Line => match command.mode {
                Mode::Flat => remaining -= 1,
                Mode::Broken => return true,
            },
            Doc::SoftLine => match command.mode {
                Mode::Flat => {}
                Mode::Broken => return true,
            },
            Doc::Group(inner) | Doc::FlexBreak(inner) => {
                probe.push(Command {
                    indent: command.indent,
                    mode: Mode::Flat,
                    doc: inner,
                });
            }
        }
    }

    false
}

fn adjust_indent(indent: usize, delta: i32) -> usize {
    if delta >= 0 {
        indent.saturating_add(delta as usize)
    } else {
        indent.saturating_sub(delta.unsigned_abs() as usize)
    }
}

fn display_width(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::{format, Doc};

    fn text(value: &str) -> Doc {
        Doc::Text(value.to_owned())
    }

    fn concat(left: Doc, right: Doc) -> Doc {
        Doc::Concat(Box::new(left), Box::new(right))
    }

    fn concat_all(mut docs: Vec<Doc>) -> Doc {
        if docs.is_empty() {
            return Doc::Nil;
        }

        let mut doc = docs.remove(0);
        for next in docs {
            doc = concat(doc, next);
        }
        doc
    }

    fn line() -> Doc {
        Doc::Line
    }

    fn soft_line() -> Doc {
        Doc::SoftLine
    }

    fn nest(indent: i32, doc: Doc) -> Doc {
        Doc::Nest(indent, Box::new(doc))
    }

    fn group(doc: Doc) -> Doc {
        Doc::Group(Box::new(doc))
    }

    fn flex(doc: Doc) -> Doc {
        Doc::FlexBreak(Box::new(doc))
    }

    #[test]
    fn group_stays_flat_when_content_fits() {
        let doc = group(concat(text("hello"), concat(line(), text("world"))));

        assert_eq!(format(&doc, 20), "hello world");
    }

    #[test]
    fn group_breaks_when_width_is_exceeded() {
        let doc = group(concat(text("hello"), concat(line(), text("world"))));

        assert_eq!(format(&doc, 10), "hello\nworld");
    }

    #[test]
    fn nest_indents_broken_lines() {
        let doc = group(concat_all(vec![
            text("foo("),
            nest(2, concat(line(), text("bar"))),
            line(),
            text(")"),
        ]));

        assert_eq!(format(&doc, 6), "foo(\n  bar\n)");
    }

    #[test]
    fn concat_and_nil_compose_stably() {
        let doc = concat_all(vec![text("a"), Doc::Nil, text("b"), Doc::Nil, text("c")]);

        assert_eq!(format(&doc, 80), "abc");
    }

    #[test]
    fn flex_break_can_stay_inline_inside_broken_group() {
        let doc = group(concat_all(vec![
            text("begin"),
            line(),
            text("abcdefghij"),
            flex(group(concat(text(","), concat(line(), text("x"))))),
            flex(group(concat(text(","), concat(line(), text("y"))))),
            line(),
            text("end"),
        ]));

        assert_eq!(format(&doc, 16), "begin\nabcdefghij, x, y\nend");
    }

    #[test]
    fn flex_break_falls_back_to_broken_layout_when_suffix_does_not_fit() {
        let doc = group(concat_all(vec![
            text("begin"),
            line(),
            text("abcdefghij"),
            flex(group(concat(text(","), concat(line(), text("x"))))),
            flex(group(concat(text(","), concat(line(), text("y"))))),
            line(),
            text("end"),
        ]));

        assert_eq!(format(&doc, 12), "begin\nabcdefghij,\nx, y\nend");
    }

    #[test]
    fn soft_line_disappears_when_group_stays_flat() {
        let doc = group(concat_all(vec![text("call("), soft_line(), text("arg"), soft_line(), text(")")]));

        assert_eq!(format(&doc, 20), "call(arg)");
        assert_eq!(format(&doc, 4), "call(\narg\n)");
    }
}
