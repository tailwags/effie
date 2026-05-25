//! UEFI firmware tables.
//!
//! Provides access to the UEFI System Table, Boot Services Table, and Runtime Services
//! Table, along with associated types such as [`Signature`], [`TableHeader`], and
//! [`SpecificationRevision`].

mod boot_services;
mod runtime_services;
mod system;

pub use boot_services::*;
pub use runtime_services::*;
pub use system::*;

use effie_macros::w_internal;

use crate::WStr;

/// A 64-bit signature that identifies the type of table that follows. Unique signatures have
/// been generated for the EFI System Table, the EFI Boot Services Table, and the EFI Runtime
/// Services Table. (UEFI specification §4.3)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Signature(pub u64);

impl Signature {
    /// EFI_SYSTEM_TABLE_SIGNATURE. Value: 0x5453595320494249 ('IBI SYST').
    pub const SYSTEM_TABLE: Self = Self(0x5453595320494249);
    /// EFI_BOOT_SERVICES_SIGNATURE. Value: 0x56524553544f4f42 ('BOOTSERV').
    pub const BOOT_SERVICES: Self = Self(0x56524553544f4f42);
}

/// Data structure that precedes all of the standard EFI table types. (UEFI specification §4.2)
#[repr(C)]
pub struct TableHeader {
    /// The 64-bit signature that identifies this table.
    pub signature: Signature,
    /// The revision of the UEFI Specification to which this table conforms.
    pub revision: SpecificationRevision,
    /// The size, in bytes, of the entire table including the header.
    pub size: u32,
    /// The 32-bit CRC for the entire table. Computed with this field set to 0.
    pub crc32: u32,
    /// Reserved field. Always set to 0
    __reserved: u32,
}

/// UEFI specification revision. Encoded as `(major << 16) | minor`. The minor field is
/// binary-coded decimal (e.g. 2.10 → 0x0002_0064, not 0x0002_000A). (UEFI specification §4.2)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SpecificationRevision(
    /// Encoded as `(major << 16) | minor`. Minor is BCD (2.10 → 0x0064).
    u32,
);

impl SpecificationRevision {
    /// UEFI specification revision 2.10.
    pub const EFI_2_100: Self = Self((2 << 16) | (100));
    /// UEFI specification revision 2.9.
    pub const EFI_2_90: Self = Self((2 << 16) | (90));
    /// UEFI specification revision 2.8.
    pub const EFI_2_80: Self = Self((2 << 16) | (80));
    /// UEFI specification revision 2.7.
    pub const EFI_2_70: Self = Self((2 << 16) | (70));
    /// UEFI specification revision 2.6.
    pub const EFI_2_60: Self = Self((2 << 16) | (60));
    /// UEFI specification revision 2.5.
    pub const EFI_2_50: Self = Self((2 << 16) | (50));
    /// UEFI specification revision 2.4.
    pub const EFI_2_40: Self = Self((2 << 16) | (40));
    /// UEFI specification revision 2.31.
    pub const EFI_2_31: Self = Self((2 << 16) | (31));
    /// UEFI specification revision 2.3.
    pub const EFI_2_30: Self = Self((2 << 16) | (30));
    /// UEFI specification revision 2.2.
    pub const EFI_2_20: Self = Self((2 << 16) | (20));
    /// UEFI specification revision 2.1.
    pub const EFI_2_10: Self = Self((2 << 16) | (10));
    /// UEFI specification revision 2.0.
    pub const EFI_2_00: Self = Self(2 << 16);
    /// UEFI specification revision 1.1.
    pub const EFI_1_10: Self = Self((1 << 16) | (10));
    /// UEFI specification revision 1.02.
    pub const EFI_1_02: Self = Self((1 << 16) | (2));
    /// The current UEFI specification revision (2.10).
    pub const EFI: Self = Self::EFI_2_100;

    /// Returns a string representation of this revision, suitable for display.
    pub const fn as_str(&self) -> &WStr {
        match *self {
            Self::EFI_2_100 => w_internal!("2.10"),
            Self::EFI_2_90 => w_internal!("2.9"),
            Self::EFI_2_80 => w_internal!("2.8"),
            Self::EFI_2_70 => w_internal!("2.7"),
            Self::EFI_2_60 => w_internal!("2.6"),
            Self::EFI_2_50 => w_internal!("2.5"),
            Self::EFI_2_40 => w_internal!("2.4"),
            Self::EFI_2_31 => w_internal!("2.3.1"),
            Self::EFI_2_30 => w_internal!("2.3"),
            Self::EFI_2_20 => w_internal!("2.2"),
            Self::EFI_2_10 => w_internal!("2.1"),
            Self::EFI_2_00 => w_internal!("2.0"),
            Self::EFI_1_10 => w_internal!("1.1"),
            Self::EFI_1_02 => w_internal!("1.0.2"),
            _ => w_internal!("unknown"),
        }
    }
}
