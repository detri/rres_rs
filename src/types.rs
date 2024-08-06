//! The `types` module contains RRES chunk data and property types.

pub struct FontGlyphInfo {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub value: i32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub advance_x: i32,
}

#[derive(Eq, PartialEq)]
pub enum ResourceDataType {
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

pub enum CompressionType {
    None = 0,
    RLE = 1,
    Deflate = 10,
    LZ4 = 20,
    LZMA2 = 30,
    QOI = 40,
}

pub enum EncryptionType {
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

pub enum TextEncoding {
    Undefined = 0,
    UTF8 = 1,
    UTF8BOM = 2,
    UTF16LE = 10,
    UTF16BE = 11,
}

pub enum CodeLang {
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

pub enum PixelFormat {
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

pub enum VertexAttribute {
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

pub enum VertexFormat {
    UByte = 0,
    Byte,
    UShort,
    Short,
    UInt,
    Int,
    HFloat,
    Float,
}

pub enum FontStyle {
    Undefined = 0,
    Regular,
    Bold,
    Italic,
}
