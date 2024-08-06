//! The `ext` module contains extension traits to help read RRES files.
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use byteorder::ReadBytesExt;

pub trait ReadCcFour {
    fn read_cc_four(&mut self) -> Result<[u8; 4], Box<dyn Error>>;
}

impl ReadCcFour for BufReader<File> {
    fn read_cc_four(&mut self) -> Result<[u8; 4], Box<dyn Error>> {
        Ok([
            self.read_u8()?,
            self.read_u8()?,
            self.read_u8()?,
            self.read_u8()?,
        ])
    }
}