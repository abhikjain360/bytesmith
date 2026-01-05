#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    UnexpectedEof,
    BadLength,
    InvalidValue,
    InvalidConst,
    ChecksumMismatch,
    Io(std::io::ErrorKind), // simple wrapper to avoid std::io dep in no_std if needed, but for now we use Kind
    Custom(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnexpectedEof => write!(f, "Unexpected EOF"),
            Error::BadLength => write!(f, "Bad Length"),
            Error::InvalidValue => write!(f, "Invalid Value"),
            Error::InvalidConst => write!(f, "Invalid Constant"),
            Error::ChecksumMismatch => write!(f, "Checksum Mismatch"),
            Error::Io(e) => write!(f, "IO Error: {:?}", e),
            Error::Custom(s) => write!(f, "Custom Error: {}", s),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.kind())
    }
}
