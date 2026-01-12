#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Len {
    pub byte: usize,
    pub bit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("not enough data: expected {expected} bytes, got {got}")]
    NotEnoughData { expected: usize, got: usize },
    #[error("unaligned length: {0:?}")]
    UnalignedLength(Len),
}

impl std::ops::Add for Len {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut byte = self.byte + other.byte;
        let mut bit = self.bit + other.bit;
        if bit >= 8 {
            byte += 1;
            bit -= 8;
        }
        Self { byte, bit }
    }
}
