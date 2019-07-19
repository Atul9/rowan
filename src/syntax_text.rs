use std::{fmt, ops};

use crate::{
    cursor::{SyntaxElement, SyntaxNode},
    SmolStr, TextRange, TextUnit,
};

#[derive(Clone)]
pub struct SyntaxText {
    node: SyntaxNode,
    range: TextRange,
}

impl SyntaxText {
    pub(crate) fn new(node: SyntaxNode) -> SyntaxText {
        let range = node.text_range();
        SyntaxText { node, range }
    }

    pub fn len(&self) -> TextUnit {
        self.range.len()
    }

    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    pub fn contains_char(&self, c: char) -> bool {
        self.try_for_each_chunk(|chunk| if chunk.contains(c) { Err(()) } else { Ok(()) }).is_err()
    }

    pub fn find_char(&self, c: char) -> Option<TextUnit> {
        let mut acc: TextUnit = 0.into();
        let res = self.try_for_each_chunk(|chunk| {
            if let Some(pos) = chunk.find(c) {
                let pos: TextUnit = (pos as u32).into();
                return Err(acc + pos);
            }
            acc += TextUnit::of_str(chunk);
            Ok(())
        });
        found(res)
    }

    pub fn char_at(&self, offset: TextUnit) -> Option<char> {
        let offset = offset.into();
        let mut start: TextUnit = 0.into();
        let res = self.try_for_each_chunk(|chunk| {
            let end = start + TextUnit::of_str(chunk);
            if start <= offset && offset < end {
                let off: usize = u32::from(offset - start) as usize;
                return Err(chunk[off..].chars().next().unwrap());
            }
            start = end;
            Ok(())
        });
        found(res)
    }

    pub fn slice<R: private::SyntaxTextRange>(&self, range: R) -> SyntaxText {
        let start = range.start().unwrap_or_default();
        let end = range.end().unwrap_or(self.len());
        assert!(start <= end);
        let len = end - start;
        let start = self.range.start() + start;
        let end = start + len;
        assert!(
            start <= end,
            "invalid slice, range: {:?}, slice: {:?}",
            self.range,
            (range.start(), range.end()),
        );
        let range = TextRange::from_to(start, end);
        assert!(
            range.is_subrange(&self.range),
            "invalid slice, range: {:?}, slice: {:?}",
            self.range,
            range,
        );
        SyntaxText { node: self.node.clone(), range }
    }

    pub fn try_fold_chunks<T, F, E>(&self, init: T, mut f: F) -> Result<T, E>
    where
        F: FnMut(T, &str) -> Result<T, E>,
    {
        self.node.descendants_with_tokens().try_fold(init, move |acc, element| {
            let res = match element {
                SyntaxElement::Token(token) => {
                    let token_range = token.text_range();
                    let range = match self.range.intersection(&token_range) {
                        None => return Ok(acc),
                        Some(it) => it,
                    };
                    let slice = if range == token_range {
                        token.text()
                    } else {
                        let range = range - token_range.start();
                        &token.text()[range]
                    };
                    f(acc, slice)?
                }
                SyntaxElement::Node(_) => acc,
            };
            Ok(res)
        })
    }

    pub fn try_for_each_chunk<F: FnMut(&str) -> Result<(), E>, E>(
        &self,
        mut f: F,
    ) -> Result<(), E> {
        self.try_fold_chunks((), move |(), chunk| f(chunk))
    }

    pub fn for_each_chunk<F: FnMut(&str)>(&self, mut f: F) {
        enum Void {}
        match self.try_for_each_chunk(|chunk| Ok::<(), Void>(f(chunk))) {
            Ok(()) => (),
            Err(void) => match void {},
        }
    }
}

fn found<T>(res: Result<(), T>) -> Option<T> {
    match res {
        Ok(()) => None,
        Err(it) => Some(it),
    }
}

impl fmt::Debug for SyntaxText {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.to_string(), f)
    }
}

impl fmt::Display for SyntaxText {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.try_for_each_chunk(|chunk| fmt::Display::fmt(chunk, f))
    }
}

impl From<SyntaxText> for String {
    fn from(text: SyntaxText) -> String {
        text.to_string()
    }
}

impl PartialEq<str> for SyntaxText {
    fn eq(&self, mut rhs: &str) -> bool {
        self.try_for_each_chunk(|chunk| {
            if !rhs.starts_with(chunk) {
                return Err(());
            }
            rhs = &rhs[chunk.len()..];
            Ok(())
        })
        .is_ok()
    }
}

impl PartialEq<&'_ str> for SyntaxText {
    fn eq(&self, rhs: &&str) -> bool {
        self == *rhs
    }
}

mod private {
    use std::ops;

    use crate::{TextRange, TextUnit};

    pub trait SyntaxTextRange {
        fn start(&self) -> Option<TextUnit>;
        fn end(&self) -> Option<TextUnit>;
    }

    impl SyntaxTextRange for TextRange {
        fn start(&self) -> Option<TextUnit> {
            Some(self.start())
        }
        fn end(&self) -> Option<TextUnit> {
            Some(self.end())
        }
    }

    impl SyntaxTextRange for ops::Range<TextUnit> {
        fn start(&self) -> Option<TextUnit> {
            Some(self.start)
        }
        fn end(&self) -> Option<TextUnit> {
            Some(self.end)
        }
    }

    impl SyntaxTextRange for ops::RangeFrom<TextUnit> {
        fn start(&self) -> Option<TextUnit> {
            Some(self.start)
        }
        fn end(&self) -> Option<TextUnit> {
            None
        }
    }

    impl SyntaxTextRange for ops::RangeTo<TextUnit> {
        fn start(&self) -> Option<TextUnit> {
            None
        }
        fn end(&self) -> Option<TextUnit> {
            Some(self.end)
        }
    }

    impl SyntaxTextRange for ops::RangeFull {
        fn start(&self) -> Option<TextUnit> {
            None
        }
        fn end(&self) -> Option<TextUnit> {
            None
        }
    }
}
