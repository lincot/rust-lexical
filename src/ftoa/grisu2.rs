//! Internal implementation of the Grisu2 algorithm.
//!
//! The optimized routines are adapted from Andrea Samoljuk's `fpconv` library,
//! which is available [here](https://github.com/night-shift/fpconv).
//!
//! The following benchmarks were run on an "Intel(R) Core(TM) i7-6560U
//! CPU @ 2.20GHz" CPU, on Fedora 28, Linux kernel version 4.18.16-200
//! (x86-64), using the lexical formatter, `dtoa::write()` or `x.to_string()`,
//! avoiding any inefficiencies in Rust string parsing for `format!(...)`
//! or `write!()` macros. The code was compiled with LTO and at an optimization
//! level of 3.
//!
//! The benchmarks with `std` were compiled using "rustc 1.29.2 (17a9dc751
//! 2018-10-05", and the `no_std` benchmarks were compiled using "rustc
//! 1.31.0-nightly (46880f41b 2018-10-15)".
//!
//! The benchmark code may be found `benches/ftoa.rs`.
//!
//! # Benchmarks
//!
//! | Type  |  lexical (ns/iter) | to_string (ns/iter)   | Relative Increase |
//! |:-----:|:------------------:|:---------------------:|:-----------------:|
//! | f32   | 1,221,025          | 2,711,290             | 2.22x             |
//! | f64   | 1,248,397          | 3,558,305             | 2.85x             |
//!
//! # Raw Benchmarks
//!
//! ```text
//! test f32_dtoa      ... bench:   1,174,070 ns/iter (+/- 442,501)
//! test f32_lexical   ... bench:   1,433,234 ns/iter (+/- 633,261)
//! test f32_ryu       ... bench:     669,828 ns/iter (+/- 192,291)
//! test f32_to_string ... bench:   3,341,733 ns/iter (+/- 1,346,744)
//! test f64_dtoa      ... bench:   1,302,522 ns/iter (+/- 364,655)
//! test f64_lexical   ... bench:   1,375,384 ns/iter (+/- 596,860)
//! test f64_ryu       ... bench:   1,015,171 ns/iter (+/- 187,552)
//! test f64_to_string ... bench:   3,900,299 ns/iter (+/- 521,956)
//! ```
//!
//! Raw Benchmarks (`no_std`)
//!
//! ```text
//! test f32_dtoa      ... bench:   1,174,070 ns/iter (+/- 442,501)
//! test f32_lexical   ... bench:   1,433,234 ns/iter (+/- 633,261)
//! test f32_ryu       ... bench:     669,828 ns/iter (+/- 192,291)
//! test f32_to_string ... bench:   3,341,733 ns/iter (+/- 1,346,744)
//! test f64_dtoa      ... bench:   1,302,522 ns/iter (+/- 364,655)
//! test f64_lexical   ... bench:   1,375,384 ns/iter (+/- 596,860)
//! test f64_ryu       ... bench:   1,015,171 ns/iter (+/- 187,552)
//! test f64_to_string ... bench:   3,900,299 ns/iter (+/- 521,956)
//! ```

// Code the generate the benchmark plot:
//  import numpy as np
//  import pandas as pd
//  import matplotlib.pyplot as plt
//  plt.style.use('ggplot')
//  lexical = np.array([1221025, 1248397]) / 1e6
//  to_string = np.array([2711290, 3558305]) / 1e6
//  index = ["f32", "f64"]
//  df = pd.DataFrame({'lexical': lexical, 'to_string': to_string}, index = index)
//  ax = df.plot.bar(rot=0)
//  ax.set_ylabel("ms/iter")
//  ax.figure.tight_layout()
//  plt.show()

use sealed::mem;
use sealed::ptr;

use super::float::{cached_grisu_power, FloatType};
use super::util::*;

// FTOA BASE10
// -----------

// LOOKUPS
const TENS: [u64; 20] = [
    10000000000000000000, 1000000000000000000, 100000000000000000,
    10000000000000000, 1000000000000000, 100000000000000,
    10000000000000, 1000000000000, 100000000000,
    10000000000, 1000000000, 100000000,
    10000000, 1000000, 100000,
    10000, 1000, 100,
    10, 1
];

// FPCONV GRISU

/// Round digit to sane approximation.
unsafe extern "C"
fn round_digit(digits: *mut u8, ndigits: isize, delta: u64, mut rem: u64, kappa: u64, frac: u64)
{
    while rem < frac && delta - rem >= kappa &&
           (rem + kappa < frac || frac - rem > rem + kappa - frac) {

        *digits.offset(ndigits - 1) -= 1;
        rem += kappa;
    }
}

/// Generate digits from upper and lower range on rounding of number.
unsafe extern "C"
fn generate_digits(fp: &FloatType, upper: &FloatType, lower: &FloatType, digits: *mut u8, k: *mut i32)
    -> i32
{
    let wfrac = upper.frac - fp.frac;
    let mut delta = upper.frac - lower.frac;

    let one = FloatType {
        frac: 1 << -upper.exp,
        exp: upper.exp,
    };

    let mut part1 = upper.frac >> -one.exp;
    let mut part2 = upper.frac & (one.frac - 1);

    let mut idx: isize = 0;
    let mut kappa: i32 = 10;
    // 1000000000
    let mut divp: *const u64 = TENS.as_ptr().add(10);
    while kappa > 0 {
        // Remember not to continue! This loop has an increment condition.
        let div = *divp;
        let digit = part1 / div;
        if digit != 0 || idx != 0 {
            *digits.offset(idx) = (digit as u8) + b'0';
            idx += 1;
        }

        part1 -= (digit as u64) * div;
        kappa -= 1;

        let tmp = (part1 <<-one.exp) + part2;
        if tmp <= delta {
            *k += kappa;
            round_digit(digits, idx, delta, tmp, div << -one.exp, wfrac);
            return idx as i32;
        }

        // Increment condition, DO NOT ADD continue.
        divp = divp.add(1);
    }

    /* 10 */
    let mut unit: *const u64 = TENS.as_ptr().add(18);

    loop {
        part2 *= 10;
        delta *= 10;
        kappa -= 1;

        let digit = part2 >> -one.exp;
        if digit != 0 || idx != 0 {
            *digits.offset(idx) = (digit as u8) + b'0';
            idx += 1;
        }

        part2 &= one.frac - 1;
        if part2 < delta {
            *k += kappa;
            round_digit(digits, idx, delta, part2, one.frac, wfrac * *unit);
            return idx as i32;
        }

        unit = unit.sub(1);
    }
}

/// Core Grisu2 algorithm for the float formatter.
unsafe extern "C" fn grisu2(d: f64, digits: *mut u8, k: *mut i32) -> i32
{
    let mut w = FloatType::from_f64(d);

    let (mut lower, mut upper) = w.normalized_boundaries();
    w.normalize();

    let mut ki: i32 = mem::uninitialized();
    let cp = cached_grisu_power(upper.exp, &mut ki);

    w     = w.fast_multiply(&cp);
    upper = upper.fast_multiply(&cp);
    lower = lower.fast_multiply(&cp);

    lower.frac += 1;
    upper.frac -= 1;

    *k = -ki;

    return generate_digits(&w, &upper, &lower, digits, k);
}

/// Write the produced digits to string.
///
/// Adds formatting for exponents, and other types of information.
unsafe extern "C" fn emit_digits(digits: *mut u8, mut ndigits: i32, dest: *mut u8, k: i32)
    -> i32
{
    let exp = k + ndigits - 1;
    let mut exp = absv!(exp);

    // write plain integer (with ".0" suffix).
    if k >= 0 && exp < (ndigits + 7) {
        let idx = ndigits as usize;
        let count = k as usize;
        ptr::copy_nonoverlapping(digits, dest, idx);
        ptr::write_bytes(dest.add(idx), b'0', count);
        ptr::copy_nonoverlapping(b".0".as_ptr(), dest.add(idx + count), 2);

        return ndigits + k + 2;
    }

    // write decimal w/o scientific notation
    if k < 0 && (k > -7 || exp < 4) {
        let mut offset = ndigits - absv!(k);
        // fp < 1.0 -> write leading zero
        if offset <= 0 {
            offset = -offset;
            *dest = b'0';
            *dest.add(1) = b'.';
            ptr::write_bytes(dest.add(2), b'0', offset as usize);
            let dst = dest.add(offset as usize + 2);
            ptr::copy_nonoverlapping(digits, dst, ndigits as usize);

            return ndigits + 2 + offset;

        } else {
            // fp > 1.0
            ptr::copy_nonoverlapping(digits, dest, offset as usize);
            *dest.offset(offset as isize) = b'.';
            let dst = dest.offset(offset as isize + 1);
            let src = digits.offset(offset as isize);
            let count = (ndigits - offset) as usize;
            ptr::copy_nonoverlapping(src, dst, count);

            return ndigits + 1;
        }
    }

    // write decimal w/ scientific notation
    ndigits = minv!(ndigits, 18);

    let mut idx: isize = 0;
    *dest.offset(idx) = *digits;
    idx += 1;

    if ndigits > 1 {
        *dest.offset(idx) = b'.';
        idx += 1;
        let dst = dest.offset(idx);
        let src = digits.add(1);
        let count = (ndigits - 1) as usize;
        ptr::copy_nonoverlapping(src, dst, count);
        idx += (ndigits - 1) as isize;
    }

    *dest.offset(idx) = exponent_notation_char(10);
    idx += 1;

    let sign: u8 = match k + ndigits - 1 < 0 {
        true    => b'-',
        false   => b'+',
    };
    *dest.offset(idx) = sign;
    idx += 1;

    let mut cent: i32 = 0;
    if exp > 99 {
        cent = exp / 100;
        *dest.offset(idx) = (cent as u8) + b'0';
        idx += 1;
        exp -= cent * 100;
    }
    if exp > 9 {
        let dec = exp / 10;
        *dest.offset(idx) = (dec as u8) + b'0';
        idx += 1;
        exp -= dec * 10;
    } else if cent != 0 {
        *dest.offset(idx) = b'0';
        idx += 1;
    }

    let shift: u8 = (exp % 10) as u8;
    *dest.offset(idx) = shift + b'0';
    idx += 1;

    idx as i32
}

unsafe extern "C" fn fpconv_dtoa(d: f64, dest: *mut u8) -> i32
{
    let mut digits: [u8; 18] = mem::uninitialized();
    let mut k: i32 = 0;
    let ndigits = grisu2(d, digits.as_mut_ptr(), &mut k);
    emit_digits(digits.as_mut_ptr(), ndigits, dest, k)
}

// F32

/// Forward to double_base10.
///
/// `f` must be non-special (NaN or infinite), non-negative,
/// and non-zero.
#[inline(always)]
pub(crate) unsafe extern "C" fn float_base10(f: f32, first: *mut u8)
    -> *mut u8
{
    double_base10(f as f64, first)
}

// F64

/// Optimized algorithm for base10 numbers.
///
/// `d` must be non-special (NaN or infinite), non-negative,
/// and non-zero.
#[inline(always)]
pub(crate) unsafe extern "C" fn double_base10(d: f64, first: *mut u8)
    -> *mut u8
{
    let len = fpconv_dtoa(d, first);
    first.offset(len as isize)
}
