use std::{fmt::Write, ops::Range};

use crate::ParseError;

/// One node of a packet dissection tree, borrowing raw bytes from the parsed
/// packet slice. `bit_range` is absolute within the root packet; `byte_range`
/// is present when the bit range is byte-aligned. `path` is dot-separated from
/// the root (e.g. `Packet.inner.b`) and is filled in by [`FieldNode::set_paths`].
#[derive(Debug, Clone, PartialEq)]
pub struct FieldNode<'a> {
    pub name: String,
    pub display_name: String,
    pub path: String,
    pub type_name: String,
    pub bit_range: Range<usize>,
    pub byte_range: Option<Range<usize>>,
    pub value: Value<'a>,
    pub status: Status,
    pub hidden: bool,
    pub children: Vec<FieldNode<'a>>,
}

/// Decoded value carried by a [`FieldNode`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Value<'a> {
    UInt(u128),
    Int(i128),
    Bool(bool),
    Bytes(&'a [u8]),
    String(String),
    EnumLabel(&'static str),
    Struct,
    Array,
    UnionVariant(&'static str),
    Absent,
    Opaque,
}

/// Parse status of a [`FieldNode`]; malformed fields carry the error while
/// the rest of the tree remains usable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Status {
    Ok,
    Error(ParseError),
    Failed(&'static str),
}

impl<'a> FieldNode<'a> {
    pub fn new(
        name: impl Into<String>,
        type_name: impl Into<String>,
        bit_range: Range<usize>,
        value: Value<'a>,
    ) -> Self {
        let name = name.into();
        Self {
            display_name: name.clone(),
            path: String::new(),
            name,
            type_name: type_name.into(),
            byte_range: byte_range_of(&bit_range),
            bit_range,
            value,
            status: Status::Ok,
            hidden: false,
            children: Vec::new(),
        }
    }

    pub fn with_children(mut self, children: Vec<FieldNode<'a>>) -> Self {
        self.children = children;
        self
    }

    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    pub fn with_bit_range(mut self, bit_range: Range<usize>) -> Self {
        self.byte_range = byte_range_of(&bit_range);
        self.bit_range = bit_range;
        self
    }

    pub fn hide(mut self) -> Self {
        self.hidden = true;
        self
    }

    pub fn renamed(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.display_name = name.clone();
        self.name = name;
        self
    }

    pub fn shifted(mut self, bits: usize) -> Self {
        self.shift(bits);
        self
    }

    fn shift(&mut self, bits: usize) {
        self.bit_range = self.bit_range.start.saturating_add(bits)
            ..self.bit_range.end.saturating_add(bits);
        self.byte_range = byte_range_of(&self.bit_range);
        for child in &mut self.children {
            child.shift(bits);
        }
    }

    pub fn set_paths(&mut self, prefix: &str) {
        self.path = if prefix.is_empty() {
            self.name.clone()
        } else {
            format!("{prefix}.{}", self.name)
        };
        for child in &mut self.children {
            child.set_paths(&self.path);
        }
    }

    /// Renders the tree as deterministic indented text, one node per line:
    /// `name: type [bit_start..bit_end) = value`, appending the error status
    /// and a `(hidden)` marker when present. Intended for snapshot tests and
    /// debugging, not UI display.
    pub fn render(&self) -> String {
        let mut out = String::new();
        self.render_into(&mut out, 0);
        out
    }

    fn render_into(&self, out: &mut String, depth: usize) {
        for _ in 0..depth {
            out.push_str("  ");
        }
        let _ = write!(
            out,
            "{}: {} [{}..{}) = {}",
            self.name,
            self.type_name,
            self.bit_range.start,
            self.bit_range.end,
            self.value.render(),
        );
        match &self.status {
            Status::Ok => {}
            Status::Error(error) => {
                let _ = write!(out, " !error: {error}");
            }
            Status::Failed(variant) => {
                let _ = write!(out, " !failed: {variant}");
            }
        }
        if self.hidden {
            out.push_str(" (hidden)");
        }
        out.push('\n');
        for child in &self.children {
            child.render_into(out, depth + 1);
        }
    }

    /// Walks the tree in pre-order and collects every node whose status is not
    /// [`Status::Ok`], pairing each node's `path` with its status. Together with
    /// the tree itself this is the "partial tree plus errors" surface a UI uses
    /// to report what could not be decoded on a malformed packet.
    pub fn errors(&self) -> Vec<(&str, &Status)> {
        let mut found = Vec::new();
        self.collect_errors(&mut found);
        found
    }

    fn collect_errors<'b>(&'b self, found: &mut Vec<(&'b str, &'b Status)>) {
        if !matches!(self.status, Status::Ok) {
            found.push((self.path.as_str(), &self.status));
        }
        for child in &self.children {
            child.collect_errors(found);
        }
    }
}

impl<'a> Value<'a> {
    pub fn bytes(data: &'a [u8], bit_range: &Range<usize>) -> Self {
        let start = (bit_range.start / 8).min(data.len());
        let end = bit_range.end.div_ceil(8).clamp(start, data.len());
        Value::Bytes(&data[start..end])
    }

    fn render(&self) -> String {
        match self {
            Value::UInt(value) => value.to_string(),
            Value::Int(value) => value.to_string(),
            Value::Bool(value) => value.to_string(),
            Value::Bytes(bytes) => {
                let hex = bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("bytes({hex})")
            }
            Value::String(value) => format!("{value:?}"),
            Value::EnumLabel(label) => (*label).to_string(),
            Value::Struct => "struct".to_string(),
            Value::Array => "array".to_string(),
            Value::UnionVariant(variant) => format!("union({variant})"),
            Value::Absent => "absent".to_string(),
            Value::Opaque => "opaque".to_string(),
        }
    }
}

fn byte_range_of(bit_range: &Range<usize>) -> Option<Range<usize>> {
    (bit_range.start.is_multiple_of(8) && bit_range.end.is_multiple_of(8))
        .then_some(bit_range.start / 8..bit_range.end / 8)
}
