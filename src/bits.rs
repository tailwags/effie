//! Bitmask newtypes for UEFI flag words.
//!
//! UEFI passes flags as plain integers, so effie exposes them as
//! `#[repr(transparent)]` newtypes. [`flags!`] declares one along with a subset
//! of the `bitflags` API and a `Debug` impl that formats like `bitflags`; it
//! replaces the `bitflags` crate.

/// Declares a `#[repr(transparent)]` bitmask newtype over an unsigned integer.
///
/// The macro supplies the `#[repr(transparent)]` itself, because every flag type
/// crosses an FFI boundary and its layout is part of the UEFI ABI. It also
/// supplies a `Debug` impl that formats like `bitflags`, so `Debug` must not be
/// in the derive list.
///
/// ```ignore
/// flags! {
///     /// Example flags.
///     #[derive(Clone, Copy, PartialEq, Eq)]
///     pub struct Example: u32 {
///         /// First flag.
///         const FIRST = 0x1;
///     }
/// }
/// ```
macro_rules! flags {
    (
        $(#[$type_attr:meta])*
        $vis:vis struct $name:ident: $bits:ty {
            $(
                $(#[$flag_attr:meta])*
                const $flag:ident = $value:expr;
            )*
        }
    ) => {
        $(#[$type_attr])*
        #[repr(transparent)]
        $vis struct $name($bits);

        impl $name {
            $(
                $(#[$flag_attr])*
                pub const $flag: Self = Self($value);
            )*

            /// No bits set.
            #[inline]
            pub const fn empty() -> Self {
                Self(0)
            }

            /// Every bit used by a declared flag.
            #[inline]
            pub const fn all() -> Self {
                Self(0 $(| ($value as $bits))*)
            }

            /// The raw bits, for passing to firmware.
            #[inline]
            pub const fn bits(&self) -> $bits {
                self.0
            }

            /// Wraps raw bits, keeping any bits that no flag declares.
            #[inline]
            pub const fn from_bits_retain(bits: $bits) -> Self {
                Self(bits)
            }

            /// Wraps raw bits, returning `None` if any bit is undeclared.
            #[inline]
            pub const fn from_bits(bits: $bits) -> Option<Self> {
                if (bits & !Self::all().0) == 0 {
                    Some(Self(bits))
                } else {
                    None
                }
            }

            /// Wraps raw bits, dropping any bit that no flag declares.
            #[inline]
            pub const fn from_bits_truncate(bits: $bits) -> Self {
                Self(bits & Self::all().0)
            }

            /// Returns whether no bits are set.
            #[inline]
            pub const fn is_empty(&self) -> bool {
                self.0 == 0
            }

            /// Returns whether every bit a declared flag uses is set.
            #[inline]
            pub const fn is_all(&self) -> bool {
                (Self::all().0 | self.0) == self.0
            }

            /// Returns whether any bit set in `other` is set in `self`.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool {
                (self.0 & other.0) != 0
            }

            /// Returns whether every bit set in `other` is set in `self`.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool {
                (self.0 & other.0) == other.0
            }

            /// Sets the bits in `other`.
            #[inline]
            pub fn insert(&mut self, other: Self) {
                self.0 |= other.0;
            }

            /// Clears the bits in `other`.
            #[inline]
            pub fn remove(&mut self, other: Self) {
                self.0 &= !other.0;
            }

            /// Flips the bits in `other`.
            #[inline]
            pub fn toggle(&mut self, other: Self) {
                self.0 ^= other.0;
            }

            /// Sets or clears the bits in `other` according to `value`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                if value {
                    self.insert(other);
                } else {
                    self.remove(other);
                }
            }

            /// The bits set in either `self` or `other`.
            #[inline]
            #[must_use]
            pub const fn union(self, other: Self) -> Self {
                Self(self.0 | other.0)
            }

            /// The bits set in both `self` and `other`.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Self {
                Self(self.0 & other.0)
            }

            /// The bits set in `self` but not in `other`.
            #[inline]
            #[must_use]
            pub const fn difference(self, other: Self) -> Self {
                Self(self.0 & !other.0)
            }

            /// The bits set in exactly one of `self` and `other`.
            #[inline]
            #[must_use]
            pub const fn symmetric_difference(self, other: Self) -> Self {
                Self(self.0 ^ other.0)
            }

            /// The declared bits that are not set in `self`.
            #[inline]
            #[must_use]
            pub const fn complement(self) -> Self {
                Self::from_bits_truncate(!self.0)
            }
        }

        impl ::core::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                // Mirrors `bitflags`: `Name(A | B | 0xf6)`, or `Name(0x0)` when
                // empty. Flags are named in declaration order. A flag is named
                // when the value contains all of its bits and it still owns a
                // bit that no earlier flag consumed, which is the rule `bitflags`
                // uses: a composite is not repeated after its parts, a partially
                // overlapping composite is still named, and zero-bit flags are
                // skipped.
                struct Named($bits);

                impl ::core::fmt::Debug for Named {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        let source = self.0;
                        let mut remaining = source;
                        let mut first = true;

                        $(
                            let bits = $value as $bits;

                            if bits != 0 && (source & bits) == bits && (remaining & bits) != 0 {
                                if !first {
                                    f.write_str(" | ")?;
                                }
                                first = false;
                                f.write_str(stringify!($flag))?;
                                remaining &= !bits;
                            }
                        )*

                        if remaining != 0 {
                            if !first {
                                f.write_str(" | ")?;
                            }
                            first = false;
                            write!(f, "{remaining:#x}")?;
                        }

                        if first {
                            f.write_str("0x0")?;
                        }

                        ::core::fmt::Result::Ok(())
                    }
                }

                f.debug_tuple(stringify!($name)).field(&Named(self.0)).finish()
            }
        }

        impl ::core::ops::BitOr for $name {
            type Output = Self;

            #[inline]
            fn bitor(self, other: Self) -> Self {
                Self(self.0 | other.0)
            }
        }

        impl ::core::ops::BitOrAssign for $name {
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.0 |= other.0;
            }
        }

        impl ::core::ops::BitAnd for $name {
            type Output = Self;

            #[inline]
            fn bitand(self, other: Self) -> Self {
                Self(self.0 & other.0)
            }
        }

        impl ::core::ops::BitAndAssign for $name {
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                self.0 &= other.0;
            }
        }

        impl ::core::ops::BitXor for $name {
            type Output = Self;

            #[inline]
            fn bitxor(self, other: Self) -> Self {
                Self(self.0 ^ other.0)
            }
        }

        impl ::core::ops::BitXorAssign for $name {
            #[inline]
            fn bitxor_assign(&mut self, other: Self) {
                self.0 ^= other.0;
            }
        }

        impl ::core::ops::Sub for $name {
            type Output = Self;

            #[inline]
            fn sub(self, other: Self) -> Self {
                Self(self.0 & !other.0)
            }
        }

        impl ::core::ops::SubAssign for $name {
            #[inline]
            fn sub_assign(&mut self, other: Self) {
                self.0 &= !other.0;
            }
        }

        impl ::core::ops::Not for $name {
            type Output = Self;

            #[inline]
            fn not(self) -> Self {
                Self::from_bits_truncate(!self.0)
            }
        }
    };
}

pub(crate) use flags;
