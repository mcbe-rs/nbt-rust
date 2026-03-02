use std::fmt;
use std::io::{Read, Write};
use std::marker::PhantomData;

use byteorder::{
    BigEndian as ByteOrderBigEndian, ByteOrder, LittleEndian as ByteOrderLittleEndian,
    ReadBytesExt, WriteBytesExt,
};
use paste::paste;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingKind {
    BigEndian,
    LittleEndian,
    NetworkLittleEndian,
}

impl EncodingKind {
    pub const fn is_network(self) -> bool {
        matches!(self, Self::NetworkLittleEndian)
    }
}

pub trait Encoding: Copy + Clone + Default + fmt::Debug + Send + Sync + 'static {
    const KIND: EncodingKind;

    fn read_i16<R: Read>(reader: &mut R) -> Result<i16>;
    fn write_i16<W: Write>(writer: &mut W, value: i16) -> Result<()>;

    fn read_i32<R: Read>(reader: &mut R) -> Result<i32>;
    fn write_i32<W: Write>(writer: &mut W, value: i32) -> Result<()>;

    fn read_i64<R: Read>(reader: &mut R) -> Result<i64>;
    fn write_i64<W: Write>(writer: &mut W, value: i64) -> Result<()>;

    fn read_f32<R: Read>(reader: &mut R) -> Result<f32>;
    fn write_f32<W: Write>(writer: &mut W, value: f32) -> Result<()>;

    fn read_f64<R: Read>(reader: &mut R) -> Result<f64>;
    fn write_f64<W: Write>(writer: &mut W, value: f64) -> Result<()>;

    fn read_string_len<R: Read>(reader: &mut R) -> Result<usize>;
    fn write_string_len<W: Write>(writer: &mut W, len: usize) -> Result<()>;

    fn read_list_len<R: Read>(reader: &mut R) -> Result<usize>;
    fn write_list_len<W: Write>(writer: &mut W, len: usize) -> Result<()>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BigEndian;

#[derive(Debug, Clone, Copy, Default)]
pub struct LittleEndian;

#[derive(Debug, Clone, Copy, Default)]
pub struct NetworkLittleEndian;

#[derive(Debug, Clone, Copy, Default)]
pub struct Codec<E: Encoding> {
    _marker: PhantomData<E>,
}

impl<E: Encoding> Codec<E> {
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    pub const fn kind(self) -> EncodingKind {
        E::KIND
    }

    pub fn read_i16<R: Read>(self, reader: &mut R) -> Result<i16> {
        E::read_i16(reader)
    }

    pub fn write_i16<W: Write>(self, writer: &mut W, value: i16) -> Result<()> {
        E::write_i16(writer, value)
    }

    pub fn read_i32<R: Read>(self, reader: &mut R) -> Result<i32> {
        E::read_i32(reader)
    }

    pub fn write_i32<W: Write>(self, writer: &mut W, value: i32) -> Result<()> {
        E::write_i32(writer, value)
    }

    pub fn read_i64<R: Read>(self, reader: &mut R) -> Result<i64> {
        E::read_i64(reader)
    }

    pub fn write_i64<W: Write>(self, writer: &mut W, value: i64) -> Result<()> {
        E::write_i64(writer, value)
    }

    pub fn read_f32<R: Read>(self, reader: &mut R) -> Result<f32> {
        E::read_f32(reader)
    }

    pub fn write_f32<W: Write>(self, writer: &mut W, value: f32) -> Result<()> {
        E::write_f32(writer, value)
    }

    pub fn read_f64<R: Read>(self, reader: &mut R) -> Result<f64> {
        E::read_f64(reader)
    }

    pub fn write_f64<W: Write>(self, writer: &mut W, value: f64) -> Result<()> {
        E::write_f64(writer, value)
    }

    pub fn read_string_len<R: Read>(self, reader: &mut R) -> Result<usize> {
        E::read_string_len(reader)
    }

    pub fn write_string_len<W: Write>(self, writer: &mut W, len: usize) -> Result<()> {
        E::write_string_len(writer, len)
    }

    pub fn read_list_len<R: Read>(self, reader: &mut R) -> Result<usize> {
        E::read_list_len(reader)
    }

    pub fn write_list_len<W: Write>(self, writer: &mut W, len: usize) -> Result<()> {
        E::write_list_len(writer, len)
    }
}

pub type BigEndianCodec = Codec<BigEndian>;
pub type LittleEndianCodec = Codec<LittleEndian>;
pub type NetworkLittleEndianCodec = Codec<NetworkLittleEndian>;

pub const BE: BigEndianCodec = Codec::new();
pub const LE: LittleEndianCodec = Codec::new();
pub const NLE: NetworkLittleEndianCodec = Codec::new();

impl Encoding for BigEndian {
    const KIND: EncodingKind = EncodingKind::BigEndian;

    fn read_i16<R: Read>(reader: &mut R) -> Result<i16> {
        read_i16_order::<_, ByteOrderBigEndian>(reader)
    }

    fn write_i16<W: Write>(writer: &mut W, value: i16) -> Result<()> {
        write_i16_order::<_, ByteOrderBigEndian>(writer, value)
    }

    fn read_i32<R: Read>(reader: &mut R) -> Result<i32> {
        read_i32_order::<_, ByteOrderBigEndian>(reader)
    }

    fn write_i32<W: Write>(writer: &mut W, value: i32) -> Result<()> {
        write_i32_order::<_, ByteOrderBigEndian>(writer, value)
    }

    fn read_i64<R: Read>(reader: &mut R) -> Result<i64> {
        read_i64_order::<_, ByteOrderBigEndian>(reader)
    }

    fn write_i64<W: Write>(writer: &mut W, value: i64) -> Result<()> {
        write_i64_order::<_, ByteOrderBigEndian>(writer, value)
    }

    fn read_f32<R: Read>(reader: &mut R) -> Result<f32> {
        read_f32_order::<_, ByteOrderBigEndian>(reader)
    }

    fn write_f32<W: Write>(writer: &mut W, value: f32) -> Result<()> {
        write_f32_order::<_, ByteOrderBigEndian>(writer, value)
    }

    fn read_f64<R: Read>(reader: &mut R) -> Result<f64> {
        read_f64_order::<_, ByteOrderBigEndian>(reader)
    }

    fn write_f64<W: Write>(writer: &mut W, value: f64) -> Result<()> {
        write_f64_order::<_, ByteOrderBigEndian>(writer, value)
    }

    fn read_string_len<R: Read>(reader: &mut R) -> Result<usize> {
        Ok(read_u16_order::<_, ByteOrderBigEndian>(reader)? as usize)
    }

    fn write_string_len<W: Write>(writer: &mut W, len: usize) -> Result<()> {
        ensure_fit("string_length", len, u16::MAX as usize)?;
        write_u16_order::<_, ByteOrderBigEndian>(writer, len as u16)
    }

    fn read_list_len<R: Read>(reader: &mut R) -> Result<usize> {
        let len = read_i32_order::<_, ByteOrderBigEndian>(reader)?;
        non_negative_len("list_length", len)
    }

    fn write_list_len<W: Write>(writer: &mut W, len: usize) -> Result<()> {
        ensure_fit("list_length", len, i32::MAX as usize)?;
        write_i32_order::<_, ByteOrderBigEndian>(writer, len as i32)
    }
}

impl Encoding for LittleEndian {
    const KIND: EncodingKind = EncodingKind::LittleEndian;

    fn read_i16<R: Read>(reader: &mut R) -> Result<i16> {
        read_i16_order::<_, ByteOrderLittleEndian>(reader)
    }

    fn write_i16<W: Write>(writer: &mut W, value: i16) -> Result<()> {
        write_i16_order::<_, ByteOrderLittleEndian>(writer, value)
    }

    fn read_i32<R: Read>(reader: &mut R) -> Result<i32> {
        read_i32_order::<_, ByteOrderLittleEndian>(reader)
    }

    fn write_i32<W: Write>(writer: &mut W, value: i32) -> Result<()> {
        write_i32_order::<_, ByteOrderLittleEndian>(writer, value)
    }

    fn read_i64<R: Read>(reader: &mut R) -> Result<i64> {
        read_i64_order::<_, ByteOrderLittleEndian>(reader)
    }

    fn write_i64<W: Write>(writer: &mut W, value: i64) -> Result<()> {
        write_i64_order::<_, ByteOrderLittleEndian>(writer, value)
    }

    fn read_f32<R: Read>(reader: &mut R) -> Result<f32> {
        read_f32_order::<_, ByteOrderLittleEndian>(reader)
    }

    fn write_f32<W: Write>(writer: &mut W, value: f32) -> Result<()> {
        write_f32_order::<_, ByteOrderLittleEndian>(writer, value)
    }

    fn read_f64<R: Read>(reader: &mut R) -> Result<f64> {
        read_f64_order::<_, ByteOrderLittleEndian>(reader)
    }

    fn write_f64<W: Write>(writer: &mut W, value: f64) -> Result<()> {
        write_f64_order::<_, ByteOrderLittleEndian>(writer, value)
    }

    fn read_string_len<R: Read>(reader: &mut R) -> Result<usize> {
        Ok(read_u16_order::<_, ByteOrderLittleEndian>(reader)? as usize)
    }

    fn write_string_len<W: Write>(writer: &mut W, len: usize) -> Result<()> {
        ensure_fit("string_length", len, u16::MAX as usize)?;
        write_u16_order::<_, ByteOrderLittleEndian>(writer, len as u16)
    }

    fn read_list_len<R: Read>(reader: &mut R) -> Result<usize> {
        let len = read_i32_order::<_, ByteOrderLittleEndian>(reader)?;
        non_negative_len("list_length", len)
    }

    fn write_list_len<W: Write>(writer: &mut W, len: usize) -> Result<()> {
        ensure_fit("list_length", len, i32::MAX as usize)?;
        write_i32_order::<_, ByteOrderLittleEndian>(writer, len as i32)
    }
}

impl Encoding for NetworkLittleEndian {
    const KIND: EncodingKind = EncodingKind::NetworkLittleEndian;

    fn read_i16<R: Read>(reader: &mut R) -> Result<i16> {
        read_i16_order::<_, ByteOrderLittleEndian>(reader)
    }

    fn write_i16<W: Write>(writer: &mut W, value: i16) -> Result<()> {
        write_i16_order::<_, ByteOrderLittleEndian>(writer, value)
    }

    fn read_i32<R: Read>(reader: &mut R) -> Result<i32> {
        read_var_i32(reader)
    }

    fn write_i32<W: Write>(writer: &mut W, value: i32) -> Result<()> {
        write_var_i32(writer, value)
    }

    fn read_i64<R: Read>(reader: &mut R) -> Result<i64> {
        read_var_i64(reader)
    }

    fn write_i64<W: Write>(writer: &mut W, value: i64) -> Result<()> {
        write_var_i64(writer, value)
    }

    fn read_f32<R: Read>(reader: &mut R) -> Result<f32> {
        read_f32_order::<_, ByteOrderLittleEndian>(reader)
    }

    fn write_f32<W: Write>(writer: &mut W, value: f32) -> Result<()> {
        write_f32_order::<_, ByteOrderLittleEndian>(writer, value)
    }

    fn read_f64<R: Read>(reader: &mut R) -> Result<f64> {
        read_f64_order::<_, ByteOrderLittleEndian>(reader)
    }

    fn write_f64<W: Write>(writer: &mut W, value: f64) -> Result<()> {
        write_f64_order::<_, ByteOrderLittleEndian>(writer, value)
    }

    fn read_string_len<R: Read>(reader: &mut R) -> Result<usize> {
        let len = read_var_u32(reader)? as usize;
        ensure_fit("string_length", len, i32::MAX as usize)?;
        Ok(len)
    }

    fn write_string_len<W: Write>(writer: &mut W, len: usize) -> Result<()> {
        ensure_fit("string_length", len, i32::MAX as usize)?;
        write_var_u32(writer, len as u32)
    }

    fn read_list_len<R: Read>(reader: &mut R) -> Result<usize> {
        let len = read_var_u32(reader)? as usize;
        ensure_fit("list_length", len, i32::MAX as usize)?;
        Ok(len)
    }

    fn write_list_len<W: Write>(writer: &mut W, len: usize) -> Result<()> {
        ensure_fit("list_length", len, i32::MAX as usize)?;
        write_var_u32(writer, len as u32)
    }
}

pub fn encode_zigzag_i32(value: i32) -> u32 {
    ((value << 1) ^ (value >> 31)) as u32
}

pub fn decode_zigzag_i32(value: u32) -> i32 {
    ((value >> 1) as i32) ^ -((value & 1) as i32)
}

pub fn encode_zigzag_i64(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}

pub fn decode_zigzag_i64(value: u64) -> i64 {
    ((value >> 1) as i64) ^ -((value & 1) as i64)
}

pub fn read_var_u32<R: Read>(reader: &mut R) -> Result<u32> {
    let mut value = 0u32;
    let mut shift = 0u32;

    for index in 0..5u8 {
        let byte = match read_u8(reader) {
            Ok(byte) => byte,
            Err(Error::Io(io_error)) if io_error.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(Error::InvalidVarint {
                    detail: "truncated u32 varint",
                });
            }
            Err(error) => return Err(error),
        };
        value |= ((byte & 0x7F) as u32) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        shift += 7;
        if index == 4 {
            return Err(Error::InvalidVarint {
                detail: "u32 varint exceeds 5 bytes",
            });
        }
    }

    Err(Error::InvalidVarint {
        detail: "failed to decode u32 varint",
    })
}

pub fn write_var_u32<W: Write>(writer: &mut W, mut value: u32) -> Result<()> {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        writer.write_all(&[byte])?;
        if value == 0 {
            return Ok(());
        }
    }
}

pub fn read_var_u64<R: Read>(reader: &mut R) -> Result<u64> {
    let mut value = 0u64;
    let mut shift = 0u32;

    for index in 0..10u8 {
        let byte = match read_u8(reader) {
            Ok(byte) => byte,
            Err(Error::Io(io_error)) if io_error.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(Error::InvalidVarint {
                    detail: "truncated u64 varint",
                });
            }
            Err(error) => return Err(error),
        };
        if index == 9 {
            if byte & 0x80 != 0 {
                return Err(Error::InvalidVarint {
                    detail: "u64 varint exceeds 10 bytes",
                });
            }
            if byte > 0x01 {
                return Err(Error::InvalidVarint {
                    detail: "u64 varint terminal byte overflow",
                });
            }
        }
        value |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        shift += 7;
        if index == 9 {
            return Err(Error::InvalidVarint {
                detail: "u64 varint exceeds 10 bytes",
            });
        }
    }

    Err(Error::InvalidVarint {
        detail: "failed to decode u64 varint",
    })
}

pub fn write_var_u64<W: Write>(writer: &mut W, mut value: u64) -> Result<()> {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        writer.write_all(&[byte])?;
        if value == 0 {
            return Ok(());
        }
    }
}

pub fn read_var_i32<R: Read>(reader: &mut R) -> Result<i32> {
    let raw = read_var_u32(reader)?;
    Ok(decode_zigzag_i32(raw))
}

pub fn write_var_i32<W: Write>(writer: &mut W, value: i32) -> Result<()> {
    write_var_u32(writer, encode_zigzag_i32(value))
}

pub fn read_var_i64<R: Read>(reader: &mut R) -> Result<i64> {
    let raw = read_var_u64(reader)?;
    Ok(decode_zigzag_i64(raw))
}

pub fn write_var_i64<W: Write>(writer: &mut W, value: i64) -> Result<()> {
    write_var_u64(writer, encode_zigzag_i64(value))
}

fn read_u8<R: Read>(reader: &mut R) -> Result<u8> {
    let mut byte = [0u8; 1];
    reader.read_exact(&mut byte)?;
    Ok(byte[0])
}

macro_rules! define_ordered_rw {
    ($(($suffix:ident, $ty:ty)),+ $(,)?) => {
        paste! {
            $(
                fn [<read_ $suffix _order>]<R: Read, O: ByteOrder>(
                    reader: &mut R,
                ) -> Result<$ty> {
                    reader.[<read_ $suffix>]::<O>().map_err(Error::from)
                }

                fn [<write_ $suffix _order>]<W: Write, O: ByteOrder>(
                    writer: &mut W,
                    value: $ty,
                ) -> Result<()> {
                    writer.[<write_ $suffix>]::<O>(value).map_err(Error::from)
                }
            )+
        }
    };
}

define_ordered_rw!(
    (u16, u16),
    (i16, i16),
    (i32, i32),
    (i64, i64),
    (f32, f32),
    (f64, f64)
);

fn ensure_fit(field: &'static str, actual: usize, max: usize) -> Result<()> {
    if actual > max {
        return Err(Error::LengthOverflow { field, max, actual });
    }
    Ok(())
}

fn non_negative_len(field: &'static str, value: i32) -> Result<usize> {
    if value < 0 {
        return Err(Error::NegativeLength { field, value });
    }
    Ok(value as usize)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn be_uses_fixed_width_i32() {
        let mut out = Vec::new();
        BE.write_i32(&mut out, 300).unwrap();
        assert_eq!(out, vec![0x00, 0x00, 0x01, 0x2c]);
    }

    #[test]
    fn le_uses_fixed_width_i32() {
        let mut out = Vec::new();
        LE.write_i32(&mut out, 300).unwrap();
        assert_eq!(out, vec![0x2c, 0x01, 0x00, 0x00]);
    }

    #[test]
    fn nle_i32_roundtrip_uses_zigzag_varint() {
        let values = [0, 1, -1, 2, -2, i32::MAX, i32::MIN];
        for value in values {
            let mut out = Vec::new();
            NLE.write_i32(&mut out, value).unwrap();
            let mut input = Cursor::new(out);
            let decoded = NLE.read_i32(&mut input).unwrap();
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn nle_i64_roundtrip_uses_zigzag_varint() {
        let values = [0, 1, -1, 2, -2, i64::MAX, i64::MIN];
        for value in values {
            let mut out = Vec::new();
            NLE.write_i64(&mut out, value).unwrap();
            let mut input = Cursor::new(out);
            let decoded = NLE.read_i64(&mut input).unwrap();
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn nle_keeps_float_as_little_endian() {
        let mut out = Vec::new();
        NLE.write_f32(&mut out, 1.5).unwrap();
        assert_eq!(out, 1.5f32.to_le_bytes());
    }

    #[test]
    fn nle_uses_varuint_for_string_and_list_lengths() {
        let mut out = Vec::new();
        NLE.write_string_len(&mut out, 300).unwrap();
        NLE.write_list_len(&mut out, 300).unwrap();

        // varuint(300) = 0xAC, 0x02
        assert_eq!(out, vec![0xAC, 0x02, 0xAC, 0x02]);

        let mut input = Cursor::new(out);
        assert_eq!(NLE.read_string_len(&mut input).unwrap(), 300);
        assert_eq!(NLE.read_list_len(&mut input).unwrap(), 300);
    }

    #[test]
    fn be_string_len_uses_u16_prefix() {
        let mut out = Vec::new();
        BE.write_string_len(&mut out, 300).unwrap();
        assert_eq!(out, vec![0x01, 0x2C]);

        let mut input = Cursor::new(out);
        assert_eq!(BE.read_string_len(&mut input).unwrap(), 300);
    }

    #[test]
    fn le_string_len_uses_u16_prefix() {
        let mut out = Vec::new();
        LE.write_string_len(&mut out, 300).unwrap();
        assert_eq!(out, vec![0x2C, 0x01]);

        let mut input = Cursor::new(out);
        assert_eq!(LE.read_string_len(&mut input).unwrap(), 300);
    }

    #[test]
    fn be_list_len_uses_i32_prefix() {
        let mut out = Vec::new();
        BE.write_list_len(&mut out, 300).unwrap();
        assert_eq!(out, vec![0x00, 0x00, 0x01, 0x2C]);

        let mut input = Cursor::new(out);
        assert_eq!(BE.read_list_len(&mut input).unwrap(), 300);
    }

    #[test]
    fn le_list_len_uses_i32_prefix() {
        let mut out = Vec::new();
        LE.write_list_len(&mut out, 300).unwrap();
        assert_eq!(out, vec![0x2C, 0x01, 0x00, 0x00]);

        let mut input = Cursor::new(out);
        assert_eq!(LE.read_list_len(&mut input).unwrap(), 300);
    }

    #[test]
    fn be_string_len_overflow_is_rejected() {
        let err = BE.write_string_len(&mut Vec::new(), (u16::MAX as usize) + 1);
        assert!(matches!(err, Err(Error::LengthOverflow { .. })));
    }

    #[test]
    fn be_negative_list_len_is_rejected() {
        let mut input = Cursor::new((-1i32).to_be_bytes());
        let err = BE.read_list_len(&mut input);
        assert!(matches!(err, Err(Error::NegativeLength { .. })));
    }

    #[test]
    fn overlong_var_u32_is_rejected() {
        let bytes = vec![0x80, 0x80, 0x80, 0x80, 0x80, 0x00];
        let mut input = Cursor::new(bytes);
        let err = read_var_u32(&mut input);
        assert!(matches!(err, Err(Error::InvalidVarint { .. })));
    }

    #[test]
    fn overlong_var_u64_is_rejected() {
        let bytes = vec![
            0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00,
        ];
        let mut input = Cursor::new(bytes);
        let err = read_var_u64(&mut input);
        assert!(matches!(err, Err(Error::InvalidVarint { .. })));
    }

    #[test]
    fn invalid_terminal_byte_var_u64_is_rejected() {
        // 10-byte payload where the terminal byte carries bits beyond bit 63.
        let bytes = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x02];
        let mut input = Cursor::new(bytes);
        let err = read_var_u64(&mut input);
        assert!(matches!(
            err,
            Err(Error::InvalidVarint {
                detail: "u64 varint terminal byte overflow"
            })
        ));
    }

    #[test]
    fn truncated_var_u32_is_rejected_as_invalid_varint() {
        let mut input = Cursor::new(vec![0x80]);
        let err = read_var_u32(&mut input);
        assert!(matches!(err, Err(Error::InvalidVarint { .. })));
    }

    #[test]
    fn truncated_var_u64_is_rejected_as_invalid_varint() {
        let mut input = Cursor::new(vec![0x80]);
        let err = read_var_u64(&mut input);
        assert!(matches!(err, Err(Error::InvalidVarint { .. })));
    }

    #[test]
    fn var_u64_boundary_values_roundtrip() {
        let values = [
            0u64,
            1,
            127,
            128,
            255,
            16_383,
            16_384,
            u32::MAX as u64,
            (u32::MAX as u64) + 1,
            (1u64 << 63) - 1,
            1u64 << 63,
            u64::MAX,
        ];

        for value in values {
            let mut out = Vec::new();
            write_var_u64(&mut out, value).unwrap();
            let mut input = Cursor::new(out);
            let decoded = read_var_u64(&mut input).unwrap();
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn nle_keeps_i16_as_little_endian_fixed_width() {
        let mut out = Vec::new();
        NLE.write_i16(&mut out, 0x1234).unwrap();
        assert_eq!(out, 0x1234i16.to_le_bytes());

        let mut input = Cursor::new(out);
        let value = NLE.read_i16(&mut input).unwrap();
        assert_eq!(value, 0x1234i16);
    }

    #[test]
    fn nle_keeps_f64_as_little_endian() {
        let mut out = Vec::new();
        NLE.write_f64(&mut out, 123.456).unwrap();
        assert_eq!(out, 123.456f64.to_le_bytes());

        let mut input = Cursor::new(out);
        let value = NLE.read_f64(&mut input).unwrap();
        assert_eq!(value, 123.456f64);
    }

    #[test]
    fn nle_read_string_len_over_i32_max_is_rejected() {
        // varuint(2_147_483_648) => 0x80 0x80 0x80 0x80 0x08
        let mut input = Cursor::new(vec![0x80, 0x80, 0x80, 0x80, 0x08]);
        let err = NLE.read_string_len(&mut input);
        assert!(matches!(err, Err(Error::LengthOverflow { .. })));
    }

    #[test]
    fn nle_write_list_len_over_i32_max_is_rejected() {
        let err = NLE.write_list_len(&mut Vec::new(), (i32::MAX as usize) + 1);
        assert!(matches!(err, Err(Error::LengthOverflow { .. })));
    }
}
