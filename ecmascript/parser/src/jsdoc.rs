//! JsDoc parser

use crate::{
    error::{Eof, Error, ErrorToDiag, SyntaxError},
    token::Token,
    PResult,
};
use swc_common::{comments::Comment, errors::Handler, BytePos, Span, SyntaxContext};
use swc_ecma_ast::Str;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsDoc {
    pub description: Str,
    pub items: Vec<JsDocItem>,
}

/// Starts with '@'
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JsDocItem {
    Simple { ty: Str, key: Str, value: Str },
}

#[derive(Clone)]
pub struct JsDocParser<'a, 'b> {
    handler: &'a Handler,
    span: Span,
    text: &'b str,
    group_fin: Option<char>,
}

impl<'a, 'b> JsDocParser<'a, 'b> {
    pub fn new(handler: &'a Handler, span: Span, text: &'b str) -> Self {
        Self {
            handler,
            span,
            text,
            group_fin: None,
        }
    }
}

impl<'a, 'b> JsDocParser<'a, 'b> {
    pub fn parse(&mut self) -> PResult<'a, JsDoc> {
        let description = self.parse_description()?;
        let items = self.parse_items()?;

        Ok(JsDoc { description, items })
    }

    pub fn parse_description(&mut self) -> PResult<'a, Str> {
        let mut buf = String::new();
        let lo = self.span.lo();
        let mut hi = self.span.lo();

        'outer: loop {
            let mut last = 0;
            for (i, c) in self.text.char_indices() {
                if c == '@' {
                    let s = self.bump(i);
                    hi = s.span.hi();
                    buf.push_str(&s.value);
                    return Ok(Str {
                        span: Span::new(lo, hi, SyntaxContext::empty()),
                        value: buf.into(),
                        has_escape: false,
                    });
                }

                if c == '\n' || c == '\r' {
                    // Remove newlines and star.
                    self.skip_ws_and_line_break();
                    let s = self.bump(i);
                    hi = s.span.hi();
                    buf.push_str(&s.value);
                    continue 'outer;
                }
            }
        }
    }

    pub fn parse_item(&mut self) -> PResult<'a, JsDocItem> {
        println!("parse_item: {}", self.text);

        if self.text.starts_with('@') {
            self.bump(1);

            let ty = self.parse_str(false, false)?;

            self.skip_ws();

            let key = self.parse_str(false, false)?;

            self.skip_ws_and_line_break();

            let value = self.parse_str(true, true)?;
            return Ok(JsDocItem::Simple { ty, key, value });
        }

        Err(ErrorToDiag {
            handler: self.handler,
            span: self.span,
            error: SyntaxError::Expected(&Token::At, self.text.to_string()),
        })?
    }

    fn parse_str(&mut self, allow_empty: bool, eat_extra: bool) -> PResult<'a, Str> {
        println!("parse_str: {}", self.text);

        self.skip_ws_and_line_break();

        if self.text.is_empty() {
            if allow_empty {
                return Ok(Str {
                    span: self.span,
                    value: Default::default(),
                    has_escape: false,
                });
            } else {
                Err(Eof {
                    last: self.span,
                    handler: self.handler,
                })?;
            };
        }

        if self.text.starts_with('(') {
            return self.parse_group(')');
        }
        if self.text.starts_with('[') {
            return self.parse_group(']');
        }
        if self.text.starts_with('{') {
            return self.parse_group('}');
        }

        for (i, c) in self.text.char_indices() {
            if let Some(fin) = self.group_fin {
                if c == fin {
                    return Ok(self.bump(i));
                }
            }

            if !eat_extra && self.group_fin.is_none() && c.is_ascii_whitespace() {
                return Ok(self.bump(i));
            }

            if eat_extra && (c == '\n' || c == '\t') {
                let ret = self.bump(i);
                self.skip_ws_and_line_break();
                return Ok(ret);
            }
        }

        Err(Eof {
            last: self.span,
            handler: self.handler,
        })?
    }

    fn parse_group(&mut self, fin: char) -> PResult<'a, Str> {
        self.bump(1);
        let old = self.group_fin;
        self.group_fin = Some(fin);

        let ret = self.parse_str(true, true)?;

        self.group_fin = old;

        Ok(ret)
    }

    pub fn parse_items(&mut self) -> PResult<'a, Vec<JsDocItem>> {
        let mut items = vec![];
        while self.text.starts_with('@') {
            self.skip_ws_and_line_break();
            items.push(self.parse_item()?);
        }

        Ok(items)
    }

    fn skip_ws(&mut self) {
        for (i, c) in self.text.char_indices() {
            if c == 'v' || c == '\t' {
                continue;
            }

            self.bump(i);
            return;
        }
    }

    fn skip_ws_and_line_break(&mut self) {
        let mut last_was_star = false;
        let mut can_eat_star = false;

        for (i, c) in self.text.char_indices() {
            if c == '\n' || c == '\r' {
                can_eat_star = true;
                continue;
            }

            if c.is_ascii_whitespace() {
                continue;
            }

            if last_was_star && c == '/' {
                continue;
            }

            if can_eat_star && c == '*' {
                can_eat_star = false;
                last_was_star = true;
                continue;
            }

            self.bump(i);
            return;
        }
    }

    fn bump(&mut self, n: usize) -> Str {
        let value = &self.text[..n];
        self.text = &self.text[n..];

        let span = if self.span.is_dummy() {
            self.span
        } else {
            let span = self.span.with_hi(self.span.lo() + BytePos(n as _));
            self.span = self.span.with_lo(self.span.lo() + BytePos(n as _));
            span
        };

        Str {
            span,
            value: value.into(),
            has_escape: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swc_common::DUMMY_SP;

    fn parse(s: &str) -> JsDoc {
        ::testing::run_test2(false, |cm, handler| {
            let mut p = JsDocParser::new(&handler, DUMMY_SP, s);

            Ok(p.parse().unwrap())
        })
        .unwrap()
    }

    fn s(s: &str) -> Str {
        Str {
            span: DUMMY_SP,
            value: s.into(),
            has_escape: false,
        }
    }

    #[test]
    fn functions() {
        let res = parse(
            "/**
 * This is a function.
 *
 * @param {string} n - A string param
 * @return {string} A good string
 *
 * @example
 *
 *     foo('hello')
 */",
        );

        assert_eq!(
            res,
            JsDoc {
                description: s("This is a function."),
                items: vec![]
            }
        )
    }
}