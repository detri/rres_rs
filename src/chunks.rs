//! The `chunks` module contains definitions for RRES resource chunks.
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use byteorder::{NativeEndian, ReadBytesExt};
use crate::{CompressionType, EncryptionType, ResourceDataType};
use crate::errors::RresError;
use crate::ext::ReadCcFour;
use crate::file::compute_crc32;

#[derive(Debug)]
pub struct ResourceChunkInfo {
    pub chunk_type: [u8; 4],
    pub chunk_id: u32,
    pub compression_type: u8,
    pub cipher_type: u8,
    pub flags: u16,
    pub packed_size: u32,
    pub base_size: u32,
    pub next_offset: u32,
    pub reserved: u32,
    pub crc32: u32,
}

impl ResourceChunkInfo {
    pub fn is_chunk_type(&self, data_type: ResourceDataType) -> bool {
        ResourceDataType::from(&self.chunk_type) == data_type
    }

    pub fn from_buf_reader(reader: &mut BufReader<File>) -> Result<ResourceChunkInfo, Box<dyn Error>> {
        let chunk_type = reader.read_cc_four()?;
        let chunk_id = reader.read_u32::<NativeEndian>()?;
        let compression_type = reader.read_u8()?;
        let cipher_type = reader.read_u8()?;
        let flags = reader.read_u16::<NativeEndian>()?;
        let packed_size = reader.read_u32::<NativeEndian>()?;
        let base_size = reader.read_u32::<NativeEndian>()?;
        let next_offset = reader.read_u32::<NativeEndian>()?;
        let reserved = reader.read_u32::<NativeEndian>()?;
        let crc32 = reader.read_u32::<NativeEndian>()?;

        Ok(ResourceChunkInfo {
            chunk_type,
            chunk_id,
            compression_type,
            cipher_type,
            flags,
            packed_size,
            base_size,
            next_offset,
            reserved,
            crc32,
        })
    }
}

impl ResourceChunkInfo {
    pub fn is_compressed_or_encrypted(&self) -> bool {
        self.compression_type != CompressionType::None as u8
            || self.cipher_type != EncryptionType::None as u8
    }
}

pub struct ResourceChunkData {
    pub prop_count: u32,
    pub props: Vec<u32>,
    pub raw_data: Vec<u8>,
}

impl ResourceChunkData {
    pub fn from_info_and_data(
        info: &ResourceChunkInfo,
        data: &mut Vec<u8>,
    ) -> Result<ResourceChunkData, Box<dyn Error>> {
        let crc32 = compute_crc32(data);
        let data_type = ResourceDataType::from(&info.chunk_type);
        match data_type {
            ResourceDataType::Null => Err(RresError::NullResource.into()),
            _ => {
                if crc32 != info.crc32 {
                    return Err(RresError::Crc32VerificationFailed.into());
                }
                return if !info.is_compressed_or_encrypted() {
                    let mut data_cursor = Cursor::new(data);
                    let prop_count = data_cursor.read_u32::<NativeEndian>().unwrap();
                    let mut props: Vec<u32> = vec![0u32; prop_count as usize];

                    if prop_count > 0 {
                        for i in 0..prop_count {
                            props[i as usize] = data_cursor.read_u32::<NativeEndian>().unwrap();
                        }
                    }

                    let raw_size = info.base_size
                        - size_of::<i32>() as u32
                        - prop_count * size_of::<i32>() as u32;
                    let mut raw_data: Vec<u8> = vec![0u8; raw_size as usize];
                    data_cursor.read_exact(&mut raw_data)?;
                    Ok(ResourceChunkData {
                        prop_count,
                        props,
                        raw_data,
                    })
                } else {
                    let prop_count = 0;
                    let props: Vec<u32> = Vec::new();
                    let mut raw_data: Vec<u8> = vec![0u8; info.packed_size as usize];
                    let mut data_cursor = Cursor::new(data);
                    data_cursor.read_exact(&mut raw_data)?;
                    Ok(ResourceChunkData {
                        prop_count,
                        props,
                        raw_data,
                    })
                };
            }
        }
    }
}

pub struct ResourceChunk {
    pub info: ResourceChunkInfo,
    pub data: ResourceChunkData,
}

pub struct ResourceMulti {
    pub chunk_count: u32,
    pub chunks: Vec<ResourceChunk>,
}