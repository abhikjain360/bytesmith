//! Protocol-specific consuming hooks referenced from the DSL specs.

use binparse::{HookContext, ParseError, ParseResult};

/// A decoded DNS name as a lazy, zero-copy view into the enclosing message.
///
/// Holds only `(enclosing, offset)`; the labels are walked on demand via
/// [`NameRef::labels`], following compression pointers, allocating nothing.
/// Labels are opaque octets on the wire (not guaranteed ASCII/UTF-8), so they
/// are yielded as raw byte slices — callers interpret them as they see fit.
#[derive(Debug, Clone, Copy)]
pub struct NameRef<'a> {
    msg: &'a [u8],
    offset: usize,
}

impl<'a> NameRef<'a> {
    /// Iterate the name's labels as borrowed byte slices, following RFC 1035
    /// §4.1.4 compression pointers. The bytes are validated when the field is
    /// parsed (see [`dns_name`]), so the iterator yields the happy path and
    /// terminates on any inconsistency rather than erroring.
    pub fn labels(&self) -> DnsLabelIter<'a> {
        DnsLabelIter {
            msg: self.msg,
            pos: self.offset,
            jumps: 0,
            done: false,
        }
    }
}

/// Lazy iterator over a [`NameRef`]'s labels.
pub struct DnsLabelIter<'a> {
    msg: &'a [u8],
    pos: usize,
    jumps: u32,
    done: bool,
}

impl<'a> Iterator for DnsLabelIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        if self.done {
            return None;
        }
        loop {
            let len_byte = *self.msg.get(self.pos)?;
            if len_byte & 0xC0 == 0xC0 {
                let second = *self.msg.get(self.pos + 1)?;
                self.jumps += 1;
                if self.jumps > 8 {
                    self.done = true;
                    return None;
                }
                self.pos = (usize::from(len_byte & 0x3F) << 8) | usize::from(second);
            } else if len_byte == 0 {
                self.done = true;
                return None;
            } else {
                let start = self.pos + 1;
                let end = start + usize::from(len_byte);
                let label = self.msg.get(start..end)?;
                self.pos = end;
                return Some(label);
            }
        }
    }
}

/// Decode a DNS name (RFC 1035 §4.1.4), following compression pointers. The
/// returned `usize` is the number of bytes consumed from the field's start (a
/// pointer terminates consumption at the pointer itself). This walks the name
/// once to validate it and measure `consumed`, but allocates nothing — the
/// returned [`NameRef`] yields the labels lazily from `ctx.enclosing`.
pub fn dns_name<'a>(_data: &[u8], ctx: HookContext<'a>) -> ParseResult<(NameRef<'a>, usize)> {
    let msg = ctx.enclosing;
    let mut pos = ctx.offset;
    let mut jumps = 0;
    loop {
        let len_byte = *msg.get(pos).ok_or(ParseError::NotEnoughData {
            expected: pos + 1,
            got: msg.len(),
        })?;
        if len_byte & 0xC0 == 0xC0 {
            let second = *msg.get(pos + 1).ok_or(ParseError::NotEnoughData {
                expected: pos + 2,
                got: msg.len(),
            })?;
            jumps += 1;
            if jumps > 8 {
                return Err(ParseError::HookFailed {
                    field: ctx.field,
                    reason: "too many DNS compression jumps",
                });
            }
            let consumed = pos + 2 - ctx.offset;
            let _ = second;
            return Ok((
                NameRef {
                    msg,
                    offset: ctx.offset,
                },
                consumed,
            ));
        } else if len_byte == 0 {
            let consumed = pos + 1 - ctx.offset;
            return Ok((
                NameRef {
                    msg,
                    offset: ctx.offset,
                },
                consumed,
            ));
        } else {
            let end = pos + 1 + usize::from(len_byte);
            msg.get(pos + 1..end).ok_or(ParseError::NotEnoughData {
                expected: end,
                got: msg.len(),
            })?;
            pos = end;
        }
    }
}
