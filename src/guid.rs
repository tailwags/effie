// Adapted from the uguid crate (https://github.com/google/gpt-disk-rs/tree/main/uguid).
// Changes: removed bytemuck and std feature support; replaced serde dependency
// with serde_core; rewrote module doc comment for effie context; updated doc
// examples to use effie paths and marked them `ignore`; updated UEFI spec link
// to version 2.11.
//
// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! UEFI GUID type.
//!
//! The GUID format follows [RFC 4122], but unlike standard UUIDs the first
//! three fields are stored little-endian as required by the UEFI Specification
//! (Appendix A). This layout is also used by Windows.
//!
//! [RFC 4122]: https://datatracker.ietf.org/doc/html/rfc4122

// `?` cannot be used in const functions; this is its replacement.
macro_rules! mtry {
    ($expr:expr $(,)?) => {
        match $expr {
            Ok(val) => val,
            Err(err) => {
                return Err(err);
            }
        }
    };
}

use core::{
    fmt::{self, Display, Formatter},
    str::{self, FromStr},
};

#[cfg(feature = "serde")]
use serde_core::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, Visitor},
};

/// Error type for [`Guid::try_parse`] and [`Guid::from_str`].
///
/// [`Guid::from_str`]: core::str::FromStr::from_str
/// [`Guid::try_parse`]: crate::Guid::try_parse
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum GuidFromStrError {
    /// Input has the wrong length, expected 36 bytes.
    #[default]
    Length,

    /// Input is missing a separator (`-`) at this byte index.
    Separator(u8),

    /// Input contains invalid ASCII hex at this byte index.
    Hex(u8),
}

impl Display for GuidFromStrError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Length => f.write_str("GUID string has wrong length (expected 36 bytes)"),
            Self::Separator(index) => {
                write!(
                    f,
                    "GUID string is missing a separator (`-`) at index {index}"
                )
            }
            Self::Hex(index) => {
                write!(f, "GUID string contains invalid ASCII hex at index {index}")
            }
        }
    }
}

impl core::error::Error for GuidFromStrError {}

/// Create a [`Guid`] from a string at compile time.
///
/// # Examples
///
/// ```ignore
/// use effie::{guid, Guid};
/// assert_eq!(
///     guid!("01234567-89ab-cdef-0123-456789abcdef"),
///     Guid::new(
///         [0x67, 0x45, 0x23, 0x01],
///         [0xab, 0x89],
///         [0xef, 0xcd],
///         0x01,
///         0x23,
///         [0x45, 0x67, 0x89, 0xab, 0xcd, 0xef],
///     )
/// );
/// ```
#[macro_export]
macro_rules! guid {
    ($s:literal) => {{
        const G: $crate::Guid = $crate::Guid::parse_or_panic($s);
        G
    }};
}

/// Globally-unique identifier.
///
/// The format is defined in [RFC 4122]. However, unlike standard UUIDs the
/// first three fields are little-endian. See also [Appendix A] of the UEFI
/// Specification.
///
/// This type is 4-byte aligned. The UEFI Specification says the GUID type
/// should be 8-byte aligned, but most C implementations use 4-byte alignment,
/// so we do the same for compatibility.
///
/// [Appendix A]: https://uefi.org/specs/UEFI/2.11/Apx_A_GUID_and_Time_Formats.html
/// [RFC 4122]: https://datatracker.ietf.org/doc/html/rfc4122
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[repr(C)]
pub struct Guid {
    // Use `u32` rather than `[u8; 4]` so the natural alignment of the struct
    // is four bytes. This is preferable to `repr(align(4))` because it does
    // not prevent use inside a `repr(packed)` struct.
    time_low: u32,
    time_mid: [u8; 2],
    time_high_and_version: [u8; 2],
    clock_seq_high_and_reserved: u8,
    clock_seq_low: u8,
    node: [u8; 6],
}

impl Guid {
    /// GUID with all fields set to zero.
    pub const ZERO: Self = Self {
        time_low: 0,
        time_mid: [0, 0],
        time_high_and_version: [0, 0],
        clock_seq_high_and_reserved: 0,
        clock_seq_low: 0,
        node: [0; 6],
    };

    /// Create a new GUID.
    #[must_use]
    pub const fn new(
        time_low: [u8; 4],
        time_mid: [u8; 2],
        time_high_and_version: [u8; 2],
        clock_seq_high_and_reserved: u8,
        clock_seq_low: u8,
        node: [u8; 6],
    ) -> Self {
        Self {
            time_low: u32::from_ne_bytes([time_low[0], time_low[1], time_low[2], time_low[3]]),
            time_mid: [time_mid[0], time_mid[1]],
            time_high_and_version: [time_high_and_version[0], time_high_and_version[1]],
            clock_seq_high_and_reserved,
            clock_seq_low,
            node,
        }
    }

    /// Create a version 4 GUID from provided random bytes.
    ///
    /// See [RFC 4122 section 4.4][rfc] for the definition of a version 4 GUID.
    ///
    /// This constructor does not itself generate random bytes; the caller must
    /// provide suitably random input.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use effie::{Guid, guid::Variant};
    ///
    /// let guid = Guid::from_random_bytes([
    ///     104, 192, 95, 215, 120, 33, 249, 1, 102, 21, 171, 84, 233, 204, 68, 176,
    /// ]);
    /// assert_eq!(guid.variant(), Variant::Rfc4122);
    /// assert_eq!(guid.version(), 4);
    /// ```
    ///
    /// [rfc]: https://datatracker.ietf.org/doc/html/rfc4122#section-4.4
    #[must_use]
    pub const fn from_random_bytes(mut random_bytes: [u8; 16]) -> Self {
        // Set the variant in byte 8: set bit 7, clear bit 6.
        random_bytes[8] &= 0b1011_1111;
        random_bytes[8] |= 0b1000_0000;
        // Set the version in byte 7: clear the top nibble, then set it to 4.
        random_bytes[7] &= 0b0000_1111;
        random_bytes[7] |= 0b0100_0000;

        Self::from_bytes(random_bytes)
    }

    /// True if all bits are zero, false otherwise.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use effie::guid;
    ///
    /// assert!(guid!("00000000-0000-0000-0000-000000000000").is_zero());
    /// assert!(!guid!("308bbc16-a308-47e8-8977-5e5646c5291f").is_zero());
    /// ```
    #[must_use]
    pub const fn is_zero(self) -> bool {
        let b = self.to_bytes();
        b[0] == 0
            && b[1] == 0
            && b[2] == 0
            && b[3] == 0
            && b[4] == 0
            && b[5] == 0
            && b[6] == 0
            && b[7] == 0
            && b[8] == 0
            && b[9] == 0
            && b[10] == 0
            && b[11] == 0
            && b[12] == 0
            && b[13] == 0
            && b[14] == 0
            && b[15] == 0
    }

    /// The little-endian low field of the timestamp.
    #[must_use]
    pub const fn time_low(self) -> [u8; 4] {
        self.time_low.to_ne_bytes()
    }

    /// The little-endian middle field of the timestamp.
    #[must_use]
    pub const fn time_mid(self) -> [u8; 2] {
        self.time_mid
    }

    /// The little-endian high field of the timestamp multiplexed with the
    /// version number.
    #[must_use]
    pub const fn time_high_and_version(self) -> [u8; 2] {
        self.time_high_and_version
    }

    /// The high field of the clock sequence multiplexed with the variant.
    #[must_use]
    pub const fn clock_seq_high_and_reserved(self) -> u8 {
        self.clock_seq_high_and_reserved
    }

    /// The low field of the clock sequence.
    #[must_use]
    pub const fn clock_seq_low(self) -> u8 {
        self.clock_seq_low
    }

    /// The spatially unique node identifier.
    #[must_use]
    pub const fn node(self) -> [u8; 6] {
        self.node
    }

    /// Get the GUID variant.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use effie::{guid, guid::Variant};
    ///
    /// assert_eq!(
    ///     guid!("308bbc16-a308-47e8-8977-5e5646c5291f").variant(),
    ///     Variant::Rfc4122
    /// );
    /// ```
    #[must_use]
    pub const fn variant(self) -> Variant {
        let bits = (self.clock_seq_high_and_reserved & 0b1110_0000) >> 5;

        if (bits & 0b100) == 0 {
            Variant::ReservedNcs
        } else if (bits & 0b010) == 0 {
            Variant::Rfc4122
        } else if (bits & 0b001) == 0 {
            Variant::ReservedMicrosoft
        } else {
            Variant::ReservedFuture
        }
    }

    /// Get the GUID version. This is a sub-type of the variant as defined in
    /// [RFC 4122 §4.1.3][rfc].
    ///
    /// # Example
    ///
    /// ```ignore
    /// use effie::guid;
    ///
    /// assert_eq!(guid!("308bbc16-a308-47e8-8977-5e5646c5291f").version(), 4);
    /// ```
    ///
    /// [rfc]: https://datatracker.ietf.org/doc/html/rfc4122#section-4.1.3
    #[must_use]
    pub const fn version(self) -> u8 {
        (self.time_high_and_version[1] & 0b1111_0000) >> 4
    }

    /// Parse a GUID from a string.
    ///
    /// This is functionally the same as [`Self::from_str`], but exposed
    /// separately to provide a `const` method for parsing.
    pub const fn try_parse(s: &str) -> Result<Self, GuidFromStrError> {
        let s = s.as_bytes();

        if s.len() != 36 {
            return Err(GuidFromStrError::Length);
        }

        let sep = b'-';
        if s[8] != sep {
            return Err(GuidFromStrError::Separator(8));
        }
        if s[13] != sep {
            return Err(GuidFromStrError::Separator(13));
        }
        if s[18] != sep {
            return Err(GuidFromStrError::Separator(18));
        }
        if s[23] != sep {
            return Err(GuidFromStrError::Separator(23));
        }

        Ok(Self::from_bytes([
            mtry!(parse_byte_at(s, 6)),
            mtry!(parse_byte_at(s, 4)),
            mtry!(parse_byte_at(s, 2)),
            mtry!(parse_byte_at(s, 0)),
            mtry!(parse_byte_at(s, 11)),
            mtry!(parse_byte_at(s, 9)),
            mtry!(parse_byte_at(s, 16)),
            mtry!(parse_byte_at(s, 14)),
            mtry!(parse_byte_at(s, 19)),
            mtry!(parse_byte_at(s, 21)),
            mtry!(parse_byte_at(s, 24)),
            mtry!(parse_byte_at(s, 26)),
            mtry!(parse_byte_at(s, 28)),
            mtry!(parse_byte_at(s, 30)),
            mtry!(parse_byte_at(s, 32)),
            mtry!(parse_byte_at(s, 34)),
        ]))
    }

    /// Parse a GUID from a string, panicking on failure.
    ///
    /// The input must be in `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` format where
    /// each `x` is a hex digit (`0-9`, `a-f`, or `A-F`).
    ///
    /// This function is marked `track_caller` so that error messages point
    /// directly to the invalid GUID string.
    ///
    /// # Panics
    ///
    /// Panics if the input is not exactly 36 bytes, is missing separators at
    /// the expected positions, or contains non-hex characters.
    #[must_use]
    #[track_caller]
    pub const fn parse_or_panic(s: &str) -> Self {
        match Self::try_parse(s) {
            Ok(g) => g,
            Err(GuidFromStrError::Length) => {
                panic!("GUID string has wrong length (expected 36 bytes)");
            }
            Err(GuidFromStrError::Separator(_)) => {
                panic!("GUID string is missing one or more separators (`-`)");
            }
            Err(GuidFromStrError::Hex(_)) => {
                panic!("GUID string contains one or more invalid characters");
            }
        }
    }

    /// Create a GUID from a 16-byte array. No byte-order conversion is applied.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self::new(
            [bytes[0], bytes[1], bytes[2], bytes[3]],
            [bytes[4], bytes[5]],
            [bytes[6], bytes[7]],
            bytes[8],
            bytes[9],
            [
                bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
            ],
        )
    }

    /// Convert to a 16-byte array.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 16] {
        let time_low = self.time_low();

        [
            time_low[0],
            time_low[1],
            time_low[2],
            time_low[3],
            self.time_mid[0],
            self.time_mid[1],
            self.time_high_and_version[0],
            self.time_high_and_version[1],
            self.clock_seq_high_and_reserved,
            self.clock_seq_low,
            self.node[0],
            self.node[1],
            self.node[2],
            self.node[3],
            self.node[4],
            self.node[5],
        ]
    }

    /// Convert to a lower-case hex ASCII string in
    /// `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` format.
    #[must_use]
    pub const fn to_ascii_hex_lower(self) -> [u8; 36] {
        let bytes = self.to_bytes();

        let mut buf = [0; 36];
        (buf[0], buf[1]) = byte_to_hex(bytes[3]);
        (buf[2], buf[3]) = byte_to_hex(bytes[2]);
        (buf[4], buf[5]) = byte_to_hex(bytes[1]);
        (buf[6], buf[7]) = byte_to_hex(bytes[0]);
        buf[8] = b'-';
        (buf[9], buf[10]) = byte_to_hex(bytes[5]);
        (buf[11], buf[12]) = byte_to_hex(bytes[4]);
        buf[13] = b'-';
        (buf[14], buf[15]) = byte_to_hex(bytes[7]);
        (buf[16], buf[17]) = byte_to_hex(bytes[6]);
        buf[18] = b'-';
        (buf[19], buf[20]) = byte_to_hex(bytes[8]);
        (buf[21], buf[22]) = byte_to_hex(bytes[9]);
        buf[23] = b'-';
        (buf[24], buf[25]) = byte_to_hex(bytes[10]);
        (buf[26], buf[27]) = byte_to_hex(bytes[11]);
        (buf[28], buf[29]) = byte_to_hex(bytes[12]);
        (buf[30], buf[31]) = byte_to_hex(bytes[13]);
        (buf[32], buf[33]) = byte_to_hex(bytes[14]);
        (buf[34], buf[35]) = byte_to_hex(bytes[15]);
        buf
    }
}

impl Default for Guid {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Display for Guid {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let ascii = self.to_ascii_hex_lower();
        // utf8: to_ascii_hex_lower only emits bytes in 0..=127.
        let s = str::from_utf8(&ascii).unwrap();
        f.write_str(s)
    }
}

impl FromStr for Guid {
    type Err = GuidFromStrError;

    /// Parse a GUID from a string, returning an error on failure.
    ///
    /// The input must be in `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` format where
    /// each `x` is a hex digit (`0-9`, `a-f`, or `A-F`).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_parse(s)
    }
}

#[cfg(feature = "serde")]
impl Serialize for Guid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let ascii = self.to_ascii_hex_lower();
        // utf8: to_ascii_hex_lower only emits bytes in 0..=127.
        let s = str::from_utf8(&ascii).unwrap();
        serializer.serialize_str(s)
    }
}

#[cfg(feature = "serde")]
struct DeserializerVisitor;

#[cfg(feature = "serde")]
impl Visitor<'_> for DeserializerVisitor {
    type Value = Guid;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string in the format \"xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx\"")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Guid::try_parse(value).map_err(E::custom)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Guid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(DeserializerVisitor)
    }
}

/// Variant or type of GUID, as defined in [RFC 4122 §4.1.1][rfc].
///
/// [rfc]: https://datatracker.ietf.org/doc/html/rfc4122#section-4.1.1
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Variant {
    /// Reserved, NCS backward compatibility.
    ReservedNcs,

    /// The GUID variant described by RFC 4122.
    Rfc4122,

    /// Reserved, Microsoft Corporation backward compatibility.
    ReservedMicrosoft,

    /// Reserved for future use.
    ReservedFuture,
}

const fn byte_to_hex(byte: u8) -> (u8, u8) {
    const fn nibble(n: u8) -> u8 {
        if n <= 9 { b'0' + n } else { b'a' - 10 + n }
    }
    (nibble(byte >> 4), nibble(byte & 0xf))
}

const fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

const fn parse_byte_at(s: &[u8], pos: u8) -> Result<u8, GuidFromStrError> {
    let i = pos as usize;
    match (hex_nibble(s[i]), hex_nibble(s[i + 1])) {
        (Some(hi), Some(lo)) => Ok((hi << 4) | lo),
        _ => Err(GuidFromStrError::Hex(pos)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_to_hex() {
        assert_eq!(byte_to_hex(0x1f), (b'1', b'f'));
        assert_eq!(byte_to_hex(0xf1), (b'f', b'1'));
    }

    #[test]
    fn test_parse_byte_at() {
        let s = b"1a8f";
        assert_eq!(parse_byte_at(s, 0), Ok(0x1a));
        assert_eq!(parse_byte_at(s, 2), Ok(0x8f));

        let bad = b"gz";
        assert_eq!(parse_byte_at(bad, 0), Err(GuidFromStrError::Hex(0)));
    }
}
