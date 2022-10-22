//! A library for Base85 encoding as described in [RFC1924](https://datatracker.ietf.org/doc/html/rfc1924) and released under the Mozilla Public License 2.0.
//!
//!## Description
//!
//! Several variants of Base85 encoding exist. The most popular variant is often known as ascii85 and is best known for use in Adobe products. This is not that algorithm.
//!
//! The variant implemented in RFC 1924 was originally intended for encoding IPv6 addresses. It utilizes the same concepts as other versions, but uses a character set which is friendly toward embedding in source code without the need for escaping. During decoding ASCII whitespace (\n, \r, \t, space) is ignored. A base85-encoded string is 25% larger than the original binary data, which is more efficient than the more-common base64 algorithm (33%). This encoding pairs very well with JSON, yielding lower overhead and needing no character escapes.
//!
//! ## Usage
//!
//! This was my first real Rust project but has matured since then and is stable. The API is simple: `encode()` turns a slice of bytes into a String and `decode()` turns a string reference into a Vector of bytes (u8). Both calls work completely within RAM, so processing huge files is probably not a good idea.
//!
//! ## Contributions
//!
//! Even though I've been coding for a while and have learned quite a bit about Rust, but I'm still a novice. Suggestions and contributions are always welcome and appreciated.

use core::mem::MaybeUninit;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unexpected end of input")]
    UnexpectedEof,
    #[error("Unexpected character '{0}'")]
    InvalidCharacter(u8),
}

#[inline]
fn byte_to_char85(x85: u8) -> u8 {
    static B85_TO_CHAR: &'static [u8] =
        b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!#$%&()*+-;<=>?@^_`{|}~";
    B85_TO_CHAR[x85 as usize]
}

#[inline]
fn char85_to_byte(c: u8) -> Result<u8> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'A'..=b'Z' => Ok(c - b'A' + 10),
        b'a'..=b'z' => Ok(c - b'a' + 36),
        b'!' => Ok(62),
        b'#' => Ok(63),
        b'$' => Ok(64),
        b'%' => Ok(65),
        b'&' => Ok(66),
        b'(' => Ok(67),
        b')' => Ok(68),
        b'*' => Ok(69),
        b'+' => Ok(70),
        b'-' => Ok(71),
        b';' => Ok(72),
        b'<' => Ok(73),
        b'=' => Ok(74),
        b'>' => Ok(75),
        b'?' => Ok(76),
        b'@' => Ok(77),
        b'^' => Ok(78),
        b'_' => Ok(79),
        b'`' => Ok(80),
        b'{' => Ok(81),
        b'|' => Ok(82),
        b'}' => Ok(83),
        b'~' => Ok(84),
        v => Err(Error::InvalidCharacter(v)),
    }
}

/// encode() turns a slice of bytes into a string of encoded data
pub fn encode(indata: &[u8]) -> String {
    if indata.len() == 0 {
        return String::from("");
    }

    let mut outdata: Vec<u8> = Vec::new();

    let length = indata.len();
    let chunk_count = (length / 4) as u32;
    let mut data_index: usize = 0;

    for _i in 0..chunk_count {
        let decnum: u32 = (indata[data_index] as u32).overflowing_shl(24).0
            | (indata[data_index + 1] as u32).overflowing_shl(16).0
            | (indata[data_index + 2] as u32).overflowing_shl(8).0
            | indata[data_index + 3] as u32;

        outdata.push(byte_to_char85((decnum as usize / 52200625) as u8));
        let mut remainder = decnum as usize % 52200625;
        outdata.push(byte_to_char85((remainder / 614125) as u8));

        remainder %= 614125;
        outdata.push(byte_to_char85((remainder / 7225) as u8));

        remainder %= 7225;
        outdata.push(byte_to_char85((remainder / 85) as u8));

        outdata.push(byte_to_char85((remainder % 85) as u8));

        data_index += 4;
    }

    let extra_bytes = length % 4;
    if extra_bytes != 0 {
        let mut last_chunk = 0_u32;

        for i in length - extra_bytes..length {
            last_chunk = last_chunk.overflowing_shl(8).0;
            last_chunk |= indata[i] as u32;
        }

        // Pad extra bytes with zeroes
        {
            let mut i = 4 - extra_bytes;
            while i > 0 {
                last_chunk = last_chunk.overflowing_shl(8).0;
                i -= 1;
            }
        }

        outdata.push(byte_to_char85((last_chunk as usize / 52200625) as u8));
        let mut remainder = last_chunk as usize % 52200625;
        outdata.push(byte_to_char85((remainder / 614125) as u8));

        if extra_bytes > 1 {
            remainder %= 614125;
            outdata.push(byte_to_char85((remainder / 7225) as u8));

            if extra_bytes > 2 {
                remainder %= 7225;
                outdata.push(byte_to_char85((remainder / 85) as u8));
            }
        }
    }

    String::from_utf8(outdata).unwrap()
}

/// decode() turns a string of encoded data into a slice of bytes
pub fn decode(instr: &str) -> Result<Vec<u8>> {
    let indata = instr.as_bytes();
    let chunks = indata.chunks_exact(5);
    let remainder = chunks.remainder();
    let capacity = if remainder.is_empty() { (indata.len()/5)*4 } else { (indata.len()/5)*4 + remainder.len()-1 };
    let mut out = Vec::<MaybeUninit<u8>>::with_capacity(capacity);
    unsafe { out.set_len(capacity); }
    let mut out_chunks = out.chunks_exact_mut(4);

    for (chunk, out_chunk) in std::iter::zip(chunks, &mut out_chunks) {
        let accumulator = u32::from(char85_to_byte(chunk[0])?) * 85u32.pow(4)
            + u32::from(char85_to_byte(chunk[1])?) * 85u32.pow(3)
            + u32::from(char85_to_byte(chunk[2])?) * 85u32.pow(2)
            + u32::from(char85_to_byte(chunk[3])?) * 85u32
            + u32::from(char85_to_byte(chunk[4])?);
        out_chunk[0] = MaybeUninit::new((accumulator >> 24) as u8);
        out_chunk[1] = MaybeUninit::new((accumulator >> 16) as u8);
        out_chunk[2] = MaybeUninit::new((accumulator >> 8) as u8);
        out_chunk[3] = MaybeUninit::new(accumulator as u8);
    }

    let out_remainder = out_chunks.into_remainder();
    if let Some(a) = remainder.first().copied() {
        let b = remainder.get(1).copied();
        let c = remainder.get(2).copied();
        let d = remainder.get(3).copied();
        let e = remainder.get(4).copied();
        let accumulator = u32::from(char85_to_byte(a)?) * 85u32.pow(4)
            + u32::from(b.map_or(Err(Error::UnexpectedEof), char85_to_byte)?) * 85u32.pow(3)
            + u32::from(c.map_or(Ok(126), char85_to_byte)?) * 85u32.pow(2)
            + u32::from(d.map_or(Ok(126), char85_to_byte)?) * 85u32.pow(1)
            + u32::from(e.map_or(Ok(126), char85_to_byte)?) * 85u32.pow(0);
        out_remainder[0] = MaybeUninit::new((accumulator >> 24) as u8);
        if remainder.len() > 2 {
            out_remainder[1] = MaybeUninit::new((accumulator >> 16) as u8);
            if remainder.len() > 3 {
                out_remainder[2] = MaybeUninit::new((accumulator >> 8) as u8);
                if remainder.len() > 4 {
                    out_remainder[3] = MaybeUninit::new(accumulator as u8);
                }
            }
        }
    }

    Ok(unsafe { std::mem::transmute::<_, Vec<u8>>(out) } )
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_encode_decode() {
        // The list of tests consists of the unencoded data on the left and the encoded data on
        // the right. By using strings for the arbitrary binary data, we make the test much less
        // complicated to write.
        let testlist = [
            ("a", "VE"),
            ("aa", "VPO"),
            ("aaa", "VPRn"),
            ("aaaa", "VPRom"),
            ("aaaaa", "VPRomVE"),
            ("aaaaaa", "VPRomVPO"),
            ("aaaaaaa", "VPRomVPRn"),
            ("aaaaaaaa", "VPRomVPRom"),
        ];

        for test in testlist.iter() {
            let s = encode(test.0.as_bytes());
            assert_eq!(
                s, test.1,
                "encoder test failed: wanted: {}, got: {}",
                test.0, s
            );

            let b = decode(test.1)
                .unwrap_or_else(|e| panic!("decoder test error on input {}: {}", test.1, e));

            let s = String::from_utf8(b).unwrap_or_else(|e| {
                panic!(
                    "decoder test '{}' failed to convert to string: {:#?}",
                    test.1, e
                )
            });

            assert_eq!(
                test.0, s,
                "decoder data mismatch: wanted: {}, got: {}",
                test.0, s
            );
        }
    }
}
