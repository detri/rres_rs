use byteorder::{NativeEndian, ReadBytesExt};
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};

trait ReadCcFour {
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

#[derive(Debug, Clone)]
enum RresError {
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

#[derive(Debug)]
struct FileHeader {
    file_id: [u8; 4],
    file_version: u16,
    chunk_count: u16,
    cd_offset: u32,
    reserved: u32,
}

impl FileHeader {
    fn from_buf_reader(reader: &mut BufReader<File>) -> Result<FileHeader, Box<dyn Error>> {
        let file_id: [u8; 4] = reader.read_cc_four()?;

        let file_version = reader.read_u16::<NativeEndian>()?;
        let chunk_count = reader.read_u16::<NativeEndian>()?;
        let cd_offset = reader.read_u32::<NativeEndian>()?;
        let reserved = reader.read_u32::<NativeEndian>()?;

        return Ok(FileHeader {
            file_id,
            file_version,
            chunk_count,
            cd_offset,
            reserved,
        });
    }

    fn verify(&self) -> bool {
        self.file_id[0] == b'r'
            && self.file_id[1] == b'r'
            && self.file_id[2] == b'e'
            && self.file_id[3] == b's'
            && self.file_version == 100
    }
}

#[derive(Debug)]
struct ResourceChunkInfo {
    chunk_type: [u8; 4],
    chunk_id: u32,
    compression_type: u8,
    cipher_type: u8,
    flags: u16,
    packed_size: u32,
    base_size: u32,
    next_offset: u32,
    reserved: u32,
    crc32: u32,
}

impl ResourceChunkInfo {
    fn is_chunk_type(&self, data_type: ResourceDataType) -> bool {
        ResourceDataType::from(&self.chunk_type) == data_type
    }

    fn from_buf_reader(reader: &mut BufReader<File>) -> Result<ResourceChunkInfo, Box<dyn Error>> {
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
    fn is_compressed_or_encrypted(&self) -> bool {
        self.compression_type != CompressionType::None as u8
            || self.cipher_type != EncryptionType::None as u8
    }
}

struct ResourceChunkData {
    prop_count: u32,
    props: Vec<u32>,
    raw_data: Vec<u8>,
}

impl ResourceChunkData {
    fn from_info_and_data(
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

struct ResourceChunk {
    info: ResourceChunkInfo,
    data: ResourceChunkData,
}

struct ResourceMulti {
    chunk_count: u32,
    chunks: Vec<ResourceChunk>,
}

#[derive(Default, Clone, Debug)]
struct DirEntry {
    resource_id: u32,
    global_offset: u32,
    reserved: u32,
    file_name_size: u32,
    file_name: Vec<u8>,
}

impl Display for DirEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.file_name.iter().take_while(|&&c| c != b'\0').map(|&c| c as char).collect::<String>())
    }
}

#[derive(Debug)]
struct CentralDir {
    entry_count: u32,
    entries: Vec<DirEntry>,
}

impl CentralDir {
    fn get_resource_id(&self, filename: String) -> u32 {
        let mut id: u32 = 0;

        for entry in &self.entries {
            if entry.to_string() == filename {
                id = entry.resource_id;
                break;
            }
        }

        id
    }
}

struct FontGlyphInfo {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    value: i32,
    offset_x: i32,
    offset_y: i32,
    advance_x: i32,
}

#[derive(Eq, PartialEq)]
enum ResourceDataType {
    Null = 0,
    Raw = 1,
    Text = 2,
    Image = 3,
    Wave = 4,
    Vertex = 5,
    FontGlyphs = 6,
    Link = 99,
    Directory = 100,
}

impl From<&[u8; 4]> for ResourceDataType {
    fn from(value: &[u8; 4]) -> Self {
        match value {
            [b'N', b'U', b'L', b'L'] => ResourceDataType::Null,
            [b'R', b'A', b'W', b'D'] => ResourceDataType::Raw,
            [b'T', b'E', b'X', b'T'] => ResourceDataType::Text,
            [b'I', b'M', b'G', b'E'] => ResourceDataType::Image,
            [b'W', b'A', b'V', b'E'] => ResourceDataType::Wave,
            [b'V', b'R', b'T', b'X'] => ResourceDataType::Vertex,
            [b'F', b'N', b'T', b'G'] => ResourceDataType::FontGlyphs,
            [b'L', b'I', b'N', b'K'] => ResourceDataType::Link,
            [b'C', b'D', b'I', b'R'] => ResourceDataType::Directory,
            _ => ResourceDataType::Null,
        }
    }
}

enum CompressionType {
    None = 0,
    RLE = 1,
    Deflate = 10,
    LZ4 = 20,
    LZMA2 = 30,
    QOI = 40,
}

enum EncryptionType {
    None = 0,
    Xor = 1,
    Des = 10,
    Tdes = 11,
    Idea = 20,
    Aes = 30,
    AesGcm = 31,
    Xtea = 40,
    Blowfish = 50,
    Rsa = 60,
    Salsa20 = 70,
    Chacha20 = 71,
    Xchacha20 = 72,
    Xchacha20Poly1305 = 73,
}

enum TextEncoding {
    Undefined = 0,
    UTF8 = 1,
    UTF8BOM = 2,
    UTF16LE = 10,
    UTF16BE = 11,
}

enum CodeLang {
    Undefined = 0,
    C,
    CPP,
    CS,
    Lua,
    JS,
    Python,
    Rust,
    Zig,
    Odin,
    Jai,
    GDScript,
    GLSL,
}

enum PixelFormat {
    Undefined = 0,
    UncompGrayscale = 1,
    UncompGrayAlpha,
    UncompR5G6B5,
    UncompR8G8B8,
    UncompR5G5B5A1,
    UncompR4G4B4A4,
    UncompR8G8B8A8,
    UncompR32,
    UncompR32G32B32,
    UncompR32G32B32A32,
    CompDxt1Rgb,
    CompDxt1Rgba,
    CompDxt3Rgba,
    CompDxt5Rgba,
    CompEtc1Rgb,
    CompEtc2Rgb,
    CompETC2EacRgba,
    CompPvrtRgb,
    CompPvrtRgba,
    CompAstc4x4Rgba,
    CompAstc8x8Rgba,
}

enum VertexAttribute {
    Position = 0,
    TexCoord1 = 10,
    TexCoord2 = 11,
    TexCoord3 = 12,
    TexCoord4 = 13,
    Normal = 20,
    Tangent = 30,
    Color = 40,
    Index = 100,
}

enum VertexFormat {
    UByte = 0,
    Byte,
    UShort,
    Short,
    UInt,
    Int,
    HFloat,
    Float,
}

enum FontStyle {
    Undefined = 0,
    Regular,
    Bold,
    Italic,
}

struct RresFile {
    filename: String,
}

impl RresFile {
    fn load_resource_chunk(&self, rres_id: u32) -> Result<ResourceChunk, Box<dyn Error>> {
        let file = File::open(&self.filename).unwrap();
        let mut reader = BufReader::new(file);

        let header = FileHeader::from_buf_reader(&mut reader)?;
        if !header.verify() {
            return Err(RresError::HeaderVerificationFailed.into());
        }

        let chunk_count = header.chunk_count;

        for _ in 0..chunk_count {
            let chunk_info = ResourceChunkInfo::from_buf_reader(&mut reader)?;
            if chunk_info.chunk_id == rres_id {
                let mut data: Vec<u8> = vec![0u8; chunk_info.packed_size as usize];
                reader.read_exact(&mut data).unwrap();

                let chunk_data = ResourceChunkData::from_info_and_data(&chunk_info, &mut data)?;
                return Ok(ResourceChunk {
                    data: chunk_data,
                    info: chunk_info,
                });
            } else {
                reader.seek(SeekFrom::Current(chunk_info.packed_size as i64))?;
            }
        }

        return Err(RresError::ChunkNotFound.into());
    }

    fn load_central_dir(&self) -> Result<CentralDir, Box<dyn Error>> {
        let file = File::open(&self.filename)?;
        let mut reader = BufReader::new(file);

        let header = FileHeader::from_buf_reader(&mut reader)?;
        if !header.verify() {
            return Err(RresError::HeaderVerificationFailed.into());
        }

        reader.seek(SeekFrom::Current(header.cd_offset as i64))?;
        let chunk_info = ResourceChunkInfo::from_buf_reader(&mut reader)?;
        dbg!(&chunk_info);
        dbg!(&header);
        if !chunk_info.is_chunk_type(ResourceDataType::Directory) {
            return Err(RresError::InvalidCentralDir.into());
        }

        let mut data: Vec<u8> = vec![0u8; chunk_info.packed_size as usize];
        reader.read_exact(&mut data)?;

        let mut chunk_data = ResourceChunkData::from_info_and_data(&chunk_info, &mut data)?;
        let entry_count = chunk_data.props[0];
        let mut entries: Vec<DirEntry> = vec![DirEntry::default(); entry_count as usize];

        let mut data_cursor = Cursor::new(&mut chunk_data.raw_data);
        for entry in entries.iter_mut() {
            entry.resource_id = data_cursor.read_u32::<NativeEndian>()?;
            entry.global_offset = data_cursor.read_u32::<NativeEndian>()?;
            data_cursor.seek(SeekFrom::Current(4))?;
            entry.file_name_size = data_cursor.read_u32::<NativeEndian>()?;
            let mut file_name: Vec<u8> = vec![0u8; entry.file_name_size as usize];
            let file_name_slice = file_name.as_mut_slice();
            data_cursor.read_exact(file_name_slice)?;
            entry.file_name = Vec::from(file_name_slice);
            println!("DIR ENTRY: {}", &entry.to_string());
        }

        Ok(CentralDir {
            entry_count,
            entries,
        })
    }
}

fn compute_crc32(data: &[u8]) -> u32 {
    static CRC_TABLE: [u32; 256] = [
        0x00000000, 0x77073096, 0xEE0E612C, 0x990951BA, 0x076DC419, 0x706AF48F, 0xE963A535,
        0x9E6495A3, 0x0eDB8832, 0x79DCB8A4, 0xE0D5E91E, 0x97D2D988, 0x09B64C2B, 0x7EB17CBD,
        0xE7B82D07, 0x90BF1D91, 0x1DB71064, 0x6AB020F2, 0xF3B97148, 0x84BE41DE, 0x1ADAD47D,
        0x6DDDE4EB, 0xF4D4B551, 0x83D385C7, 0x136C9856, 0x646BA8C0, 0xFD62F97A, 0x8A65C9EC,
        0x14015C4F, 0x63066CD9, 0xFA0F3D63, 0x8D080DF5, 0x3B6E20C8, 0x4C69105E, 0xD56041E4,
        0xA2677172, 0x3C03E4D1, 0x4B04D447, 0xD20D85FD, 0xA50AB56B, 0x35B5A8FA, 0x42B2986C,
        0xDBBBC9D6, 0xACBCF940, 0x32D86CE3, 0x45DF5C75, 0xDCD60DCF, 0xABD13D59, 0x26D930AC,
        0x51DE003A, 0xC8D75180, 0xBFD06116, 0x21B4F4B5, 0x56B3C423, 0xCFBA9599, 0xB8BDA50F,
        0x2802B89E, 0x5F058808, 0xC60CD9B2, 0xB10BE924, 0x2F6F7C87, 0x58684C11, 0xC1611DAB,
        0xB6662D3D, 0x76DC4190, 0x01DB7106, 0x98D220BC, 0xEFD5102A, 0x71B18589, 0x06B6B51F,
        0x9FBFE4A5, 0xE8B8D433, 0x7807C9A2, 0x0F00F934, 0x9609A88E, 0xE10E9818, 0x7F6A0DBB,
        0x086D3D2D, 0x91646C97, 0xE6635C01, 0x6B6B51F4, 0x1C6C6162, 0x856530D8, 0xF262004E,
        0x6C0695ED, 0x1B01A57B, 0x8208F4C1, 0xF50FC457, 0x65B0D9C6, 0x12B7E950, 0x8BBEB8EA,
        0xFCB9887C, 0x62DD1DDF, 0x15DA2D49, 0x8CD37CF3, 0xFBD44C65, 0x4DB26158, 0x3AB551CE,
        0xA3BC0074, 0xD4BB30E2, 0x4ADFA541, 0x3DD895D7, 0xA4D1C46D, 0xD3D6F4FB, 0x4369E96A,
        0x346ED9FC, 0xAD678846, 0xDA60B8D0, 0x44042D73, 0x33031DE5, 0xAA0A4C5F, 0xDD0D7CC9,
        0x5005713C, 0x270241AA, 0xBE0B1010, 0xC90C2086, 0x5768B525, 0x206F85B3, 0xB966D409,
        0xCE61E49F, 0x5EDEF90E, 0x29D9C998, 0xB0D09822, 0xC7D7A8B4, 0x59B33D17, 0x2EB40D81,
        0xB7BD5C3B, 0xC0BA6CAD, 0xEDB88320, 0x9ABFB3B6, 0x03B6E20C, 0x74B1D29A, 0xEAD54739,
        0x9DD277AF, 0x04DB2615, 0x73DC1683, 0xE3630B12, 0x94643B84, 0x0D6D6A3E, 0x7A6A5AA8,
        0xE40ECF0B, 0x9309FF9D, 0x0A00AE27, 0x7D079EB1, 0xF00F9344, 0x8708A3D2, 0x1E01F268,
        0x6906C2FE, 0xF762575D, 0x806567CB, 0x196C3671, 0x6E6B06E7, 0xFED41B76, 0x89D32BE0,
        0x10DA7A5A, 0x67DD4ACC, 0xF9B9DF6F, 0x8EBEEFF9, 0x17B7BE43, 0x60B08ED5, 0xD6D6A3E8,
        0xA1D1937E, 0x38D8C2C4, 0x4FDFF252, 0xD1BB67F1, 0xA6BC5767, 0x3FB506DD, 0x48B2364B,
        0xD80D2BDA, 0xAF0A1B4C, 0x36034AF6, 0x41047A60, 0xDF60EFC3, 0xA867DF55, 0x316E8EEF,
        0x4669BE79, 0xCB61B38C, 0xBC66831A, 0x256FD2A0, 0x5268E236, 0xCC0C7795, 0xBB0B4703,
        0x220216B9, 0x5505262F, 0xC5BA3BBE, 0xB2BD0B28, 0x2BB45A92, 0x5CB36A04, 0xC2D7FFA7,
        0xB5D0CF31, 0x2CD99E8B, 0x5BDEAE1D, 0x9B64C2B0, 0xEC63F226, 0x756AA39C, 0x026D930A,
        0x9C0906A9, 0xEB0E363F, 0x72076785, 0x05005713, 0x95BF4A82, 0xE2B87A14, 0x7BB12BAE,
        0x0CB61B38, 0x92D28E9B, 0xE5D5BE0D, 0x7CDCEFB7, 0x0BDBDF21, 0x86D3D2D4, 0xF1D4E242,
        0x68DDB3F8, 0x1FDA836E, 0x81BE16CD, 0xF6B9265B, 0x6FB077E1, 0x18B74777, 0x88085AE6,
        0xFF0F6A70, 0x66063BCA, 0x11010B5C, 0x8F659EFF, 0xF862AE69, 0x616BFFD3, 0x166CCF45,
        0xA00AE278, 0xD70DD2EE, 0x4E048354, 0x3903B3C2, 0xA7672661, 0xD06016F7, 0x4969474D,
        0x3E6E77DB, 0xAED16A4A, 0xD9D65ADC, 0x40DF0B66, 0x37D83BF0, 0xA9BCAE53, 0xDEBB9EC5,
        0x47B2CF7F, 0x30B5FFE9, 0xBDBDF21C, 0xCABAC28A, 0x53B39330, 0x24B4A3A6, 0xBAD03605,
        0xCDD70693, 0x54DE5729, 0x23D967BF, 0xB3667A2E, 0xC4614AB8, 0x5D681B02, 0x2A6F2B94,
        0xB40BBE37, 0xC30C8EA1, 0x5A05DF1B, 0x2D02EF8D,
    ];

    let mut crc: u32 = !0;

    for &byte in data.iter() {
        crc = (crc >> 8) ^ CRC_TABLE[(byte ^ (crc as u8)) as usize];
    }

    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;

    #[test]
    fn reads_central_dir() {
        let rres_file = RresFile {
            filename: "examples/resources.rres".into(),
        };
        let central_dir_result = rres_file.load_central_dir();
        match &central_dir_result {
            Ok(central_dir) => assert!(central_dir.entry_count > 0),
            Err(err) => println!("{}", err),
        }
        assert!(central_dir_result.is_ok());
    }

    #[test]
    fn reads_resource_id() {
        let rres_file = RresFile { filename: "examples/resources.rres".into() };
        let central_dir = rres_file.load_central_dir().unwrap();
        let resource_id = central_dir.get_resource_id("resources/text_data.txt".into());
        assert_eq!(resource_id, 3342539433);
    }

    #[test]
    fn reads_resource_chunk() {
        let rres_file = RresFile { filename: "examples/resources.rres".into() };
        let central_dir = rres_file.load_central_dir().unwrap();
        let resource_id = central_dir.get_resource_id("resources/text_data.txt".into());
        let chunk = rres_file.load_resource_chunk(resource_id).unwrap();
        let chunk_string = chunk.data.raw_data.iter().map(|&c| c as char).collect::<String>();
        assert_eq!(chunk_string, "Hello World! This is a test!");
    }
}
