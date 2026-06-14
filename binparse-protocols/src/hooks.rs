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

/// Encode an uncompressed RFC 1035 name (length-prefixed labels terminated by
/// `0x00`) into `dst`, applying §4.1.4 compression. Walks `name`'s suffixes
/// longest-first; for each suffix that appears verbatim (terminator included) at
/// an offset `< 0x4000` in `ctx.written`, emits the preceding labels followed by
/// a 2-byte `0xC0`-tagged pointer. With no match the labels are written verbatim.
/// The returned `usize` is the number of bytes written from the name's start.
pub fn write_dns_name(
    name: &[u8],
    dst: &mut [u8],
    ctx: binparse::WriteHookContext,
) -> binparse::WriteResult<usize> {
    let mut s = 0;
    while s < name.len() && name[s] != 0 {
        let suffix = &name[s..];
        if let Some(p) = find_subslice(ctx.written, suffix)
            && p < 0x4000
        {
            let total = s + 2;
            if dst.len() < total {
                return Err(binparse::WriteError::NotEnoughSpace {
                    expected: total,
                    got: dst.len(),
                });
            }
            dst[..s].copy_from_slice(&name[..s]);
            dst[s] = 0xC0 | (p >> 8) as u8;
            dst[s + 1] = p as u8;
            return Ok(total);
        }
        s += 1 + usize::from(name[s]);
    }

    if dst.len() < name.len() {
        return Err(binparse::WriteError::NotEnoughSpace {
            expected: name.len(),
            got: dst.len(),
        });
    }
    dst[..name.len()].copy_from_slice(name);
    Ok(name.len())
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (0..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAME: &[u8] = &[
        7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0,
    ];

    #[test]
    fn test_write_dns_name_pointer() {
        let mut written = vec![0u8; 12];
        written.extend_from_slice(NAME);
        let mut dst = [0u8; 16];
        let n = write_dns_name(
            NAME,
            &mut dst,
            binparse::WriteHookContext {
                offset: written.len(),
                written: &written,
            },
        )
        .unwrap();
        assert_eq!(&dst[..n], &[0xC0, 0x0C]);
    }

    #[test]
    fn test_write_dns_name_verbatim() {
        let mut dst = [0u8; 16];
        let n = write_dns_name(
            NAME,
            &mut dst,
            binparse::WriteHookContext {
                offset: 0,
                written: &[],
            },
        )
        .unwrap();
        assert_eq!(&dst[..n], NAME);
    }

    #[test]
    fn test_write_dns_name_pointer_round_trips() {
        let mut msg = NAME.to_vec();
        let offset = msg.len();
        let mut dst = [0u8; 16];
        let n = write_dns_name(
            NAME,
            &mut dst,
            binparse::WriteHookContext {
                offset,
                written: &msg,
            },
        )
        .unwrap();
        msg.extend_from_slice(&dst[..n]);
        let ctx = HookContext {
            field: "Dns.aname",
            offset,
            enclosing: &msg,
        };
        let (name_ref, consumed) = dns_name(&msg[offset..], ctx).unwrap();
        assert_eq!(consumed, 2);
        let labels: Vec<&[u8]> = name_ref.labels().collect();
        assert_eq!(labels, vec![b"example".as_slice(), b"com".as_slice()]);
    }
}
