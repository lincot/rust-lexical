//! Implements the algorithm in terms of the lexical API.

#![doc(hidden)]

use crate::options::Options;
use crate::write::WriteInteger;
use lexical_util::assert::{assert_buffer, debug_assert_buffer};
use lexical_util::constants::FormattedSize;
use lexical_util::format::{NumberFormat, STANDARD};
use lexical_util::num::SignedInteger;
use lexical_util::{to_lexical, to_lexical_with_options};

// UNSIGNED

/// Callback for unsigned integer formatter.
///
/// # Safety
///
/// Safe as long as the buffer can hold `FORMATTED_SIZE` elements
/// (or `FORMATTED_SIZE_DECIMAL` for decimal).
#[inline]
unsafe fn unsigned<T: WriteInteger, const FORMAT: u128>(value: T, buffer: &mut [u8]) -> usize {
    let format = NumberFormat::<FORMAT> {};
    if cfg!(feature = "format") && format.required_mantissa_sign() {
        // SAFETY: safe as long as there is at least `FORMATTED_SIZE` elements.
        unsafe {
            index_unchecked_mut!(buffer[0]) = b'+';
            let buffer = &mut index_unchecked_mut!(buffer[1..]);
            value.write_mantissa::<FORMAT>(buffer) + 1
        }
    } else {
        // SAFETY: safe as long as there is at least `FORMATTED_SIZE` elements.
        unsafe { value.write_mantissa::<FORMAT>(buffer) }
    }
}

// SIGNED

/// Callback for signed integer formatter.
///
/// # Safety
///
/// Safe as long as the buffer can hold `FORMATTED_SIZE` elements
/// (or `FORMATTED_SIZE_DECIMAL` for decimal).
#[inline]
unsafe fn signed<T: SignedInteger, const FORMAT: u128>(value: T, buffer: &mut [u8]) -> usize
where
    T::Unsigned: WriteInteger,
{
    let format = NumberFormat::<FORMAT> {};
    let unsigned = value.unsigned_abs();
    if value < T::ZERO {
        // SAFETY: safe as long as there is at least `FORMATTED_SIZE` elements.
        unsafe {
            index_unchecked_mut!(buffer[0]) = b'-';
            let buffer = &mut index_unchecked_mut!(buffer[1..]);
            unsigned.write_mantissa::<FORMAT>(buffer) + 1
        }
    } else if cfg!(feature = "format") && format.required_mantissa_sign() {
        // SAFETY: safe as long as there is at least `FORMATTED_SIZE` elements.
        unsafe {
            index_unchecked_mut!(buffer[0]) = b'+';
            let buffer = &mut index_unchecked_mut!(buffer[1..]);
            unsigned.write_mantissa::<FORMAT>(buffer) + 1
        }
    } else {
        // SAFETY: safe as long as there is at least `FORMATTED_SIZE` elements.
        unsafe { unsigned.write_mantissa::<FORMAT>(buffer) }
    }
}

// API

to_lexical! {}
to_lexical_with_options! {}

impl<T: WriteInteger + FormattedSize> ToLexical for T {
    unsafe fn to_lexical_unchecked(self, bytes: &mut [u8]) -> &mut [u8] {
        debug_assert_buffer::<T>(10, bytes.len());
        // SAFETY: safe if `bytes.len() > Self::FORMATTED_SIZE_DECIMAL`.
        unsafe {
            let len = unsigned::<T, { STANDARD }>(self, bytes);
            &mut index_unchecked_mut!(bytes[..len])
        }
    }

    fn to_lexical(self, bytes: &mut [u8]) -> &mut [u8] {
        assert_buffer::<T>(10, bytes.len());
        // SAFETY: safe since `bytes.len() > Self::FORMATTED_SIZE_DECIMAL`.
        unsafe { self.to_lexical_unchecked(bytes) }
    }
}

impl<T: WriteInteger + FormattedSize> ToLexicalWithOptions for T {
    type Options = Options;

    unsafe fn to_lexical_with_options_unchecked<'a, const FORMAT: u128>(
        self,
        bytes: &'a mut [u8],
        _: &Self::Options,
    ) -> &'a mut [u8] {
        debug_assert_buffer::<T>(NumberFormat::<{ FORMAT }>::RADIX, bytes.len());
        assert!(NumberFormat::<{ FORMAT }> {}.is_valid());
        // SAFETY: safe if `bytes.len() > Self::FORMATTED_SIZE`.
        unsafe {
            let len = unsigned::<T, FORMAT>(self, bytes);
            &mut index_unchecked_mut!(bytes[..len])
        }
    }

    fn to_lexical_with_options<'a, const FORMAT: u128>(
        self,
        bytes: &'a mut [u8],
        options: &Self::Options,
    ) -> &'a mut [u8] {
        assert_buffer::<T>(NumberFormat::<{ FORMAT }>::RADIX, bytes.len());
        assert!(NumberFormat::<{ FORMAT }> {}.is_valid());
        // SAFETY: safe since `bytes.len() > Self::FORMATTED_SIZE`.
        unsafe { self.to_lexical_with_options_unchecked::<FORMAT>(bytes, options) }
    }
}

// Implement ToLexical for numeric type.
macro_rules! signed_to_lexical {
    ($($t:tt)*) => ($(
        impl ToLexical for $t {
            unsafe fn to_lexical_unchecked(self, bytes: &mut [u8]) -> &mut [u8] {
                debug_assert_buffer::<$t>(10, bytes.len());
                // SAFETY: safe if `bytes.len() > Self::FORMATTED_SIZE_DECIMAL`.
                unsafe {
                    let len = signed::<$t, { STANDARD }>(self, bytes);
                    &mut index_unchecked_mut!(bytes[..len])
                }
            }

            fn to_lexical(self, bytes: &mut [u8]) -> &mut [u8] {
                assert_buffer::<$t>(10, bytes.len());
                // SAFETY: safe since `bytes.len() > Self::FORMATTED_SIZE_DECIMAL`.
                unsafe { self.to_lexical_unchecked(bytes) }
            }
        }

        impl ToLexicalWithOptions for $t {
            type Options = Options;

            unsafe fn to_lexical_with_options_unchecked<'a, const FORMAT: u128>(
                self,
                bytes: &'a mut [u8],
                _: &Self::Options,
            ) -> &'a mut [u8]
            {
                debug_assert_buffer::<$t>(NumberFormat::<{ FORMAT }>::RADIX, bytes.len());
                assert!(NumberFormat::<{ FORMAT }> {}.is_valid());
                // SAFETY: safe if `bytes.len() > Self::FORMATTED_SIZE`.
                unsafe {
                    let len = signed::<$t, FORMAT>(self, bytes);
                    &mut index_unchecked_mut!(bytes[..len])
                }
            }

            fn to_lexical_with_options<'a, const FORMAT: u128>(
                self,
                bytes: &'a mut [u8],
                options: &Self::Options,
            ) -> &'a mut [u8]
            {
                assert_buffer::<$t>(NumberFormat::<{ FORMAT }>::RADIX, bytes.len());
                assert!(NumberFormat::<{ FORMAT }> {}.is_valid());
                // SAFETY: safe since `bytes.len() > Self::FORMATTED_SIZE`.
                unsafe { self.to_lexical_with_options_unchecked::<FORMAT>(bytes, options) }
            }
        }
    )*)
}

signed_to_lexical! { i8 i16 i32 i64 i128 isize }
