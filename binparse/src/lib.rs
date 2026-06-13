use std::ops::{Add, Mul};

pub mod hooks;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Len {
    pub byte: usize,
    pub bit: usize,
}

impl Len {
    pub const ZERO: Self = Self { byte: 0, bit: 0 };

    pub fn bits(self) -> usize {
        self.byte.saturating_mul(8).saturating_add(self.bit)
    }

    pub fn byte_ceil(self) -> usize {
        self.byte.saturating_add(usize::from(self.bit > 0))
    }

    pub fn is_byte_aligned(self) -> bool {
        self.bit == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("not enough data: expected {expected} bytes, got {got}")]
    NotEnoughData { expected: usize, got: usize },
    #[error("unaligned length: {0:?}")]
    UnalignedLength(Len),
    #[error("validation failed for field '{field}': actual value {actual}")]
    ValidationFailed { field: &'static str, actual: u128 },
}

pub type ParseResult<T> = std::result::Result<T, ParseError>;

impl Add for Len {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut byte = self.byte.saturating_add(other.byte);
        let mut bit = self.bit + other.bit;
        if bit >= 8 {
            byte = byte.saturating_add(1);
            bit -= 8;
        }
        Self { byte, bit }
    }
}

impl Mul<usize> for Len {
    type Output = Self;

    fn mul(self, rhs: usize) -> Self::Output {
        let bit = self.bit.saturating_mul(rhs);
        Self {
            byte: self
                .byte
                .saturating_mul(rhs)
                .saturating_add(bit / 8),
            bit: bit % 8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn len_bits_and_byte_ceil() {
        let len = Len { byte: 3, bit: 5 };
        assert_eq!(len.bits(), 29);
        assert_eq!(len.byte_ceil(), 4);
        assert!(!len.is_byte_aligned());
        assert_eq!(Len { byte: 2, bit: 0 }.byte_ceil(), 2);
        assert!(Len::ZERO.is_byte_aligned());
        assert_eq!(Len::ZERO.bits(), 0);
    }

    #[test]
    fn len_add_carries_bits() {
        let a = Len { byte: 1, bit: 5 };
        let b = Len { byte: 2, bit: 6 };
        assert_eq!(a + b, Len { byte: 4, bit: 3 });
    }

    #[test]
    fn len_mul_carries_bits() {
        let len = Len { byte: 1, bit: 3 };
        assert_eq!(len * 5, Len { byte: 6, bit: 7 });
    }

    #[test]
    fn len_arithmetic_saturates_instead_of_overflowing() {
        let max = Len {
            byte: usize::MAX,
            bit: 7,
        };
        assert_eq!(
            max + max,
            Len {
                byte: usize::MAX,
                bit: 6
            }
        );
        let big = Len {
            byte: usize::MAX / 2,
            bit: 0,
        };
        assert_eq!(big * 4, Len { byte: usize::MAX, bit: 0 });
        assert_eq!(max.bits(), usize::MAX);
        assert_eq!(max.byte_ceil(), usize::MAX);
    }
}
