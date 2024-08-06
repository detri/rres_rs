use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug, Clone)]
pub enum RresError {
    NullResource,
    HeaderRead,
    ChunkNotFound,
    HeaderVerificationFailed,
    InvalidCentralDir,
    Crc32VerificationFailed,
}

impl Display for RresError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match *self {
            RresError::NullResource => {
                write!(f, "RRES: Chunk contains no data!")
            }
            RresError::HeaderRead => {
                write!(f, "RRES: Could not read rres file header!")
            }
            RresError::ChunkNotFound => {
                write!(f, "RRES: Chunk not found in file!")
            }
            RresError::HeaderVerificationFailed => {
                write!(f, "RRES: File is not an rres file!")
            }
            RresError::InvalidCentralDir => {
                write!(f, "RRES: Central directory chunk byte offset does not point to a central directory chunk!")
            }
            RresError::Crc32VerificationFailed => {
                write!(
                    f,
                    "RRES: CRC32 does not match. Data was unable to be loaded!"
                )
            }
        }
    }
}

impl Error for RresError {}
