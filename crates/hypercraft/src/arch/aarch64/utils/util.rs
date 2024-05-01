// Copyright (c) 2023 Beihang University, Huawei Technologies Co.,Ltd. All rights reserved.
// Rust-Shyper is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//          http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
// EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
// MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

use alloc::sync::Arc;
use core::any::Any;

static TRACE: AtomicBool = AtomicBool::new(true);

#[inline(always)]
/// Rounds up a value to the nearest multiple of `to`.
///
/// # Arguments
///
/// * `value` - The value to be rounded up.
/// * `to` - The multiple to round up to.
///
/// # Returns
///
/// The rounded up value.
pub fn round_up(value: usize, to: usize) -> usize {
    ((value + to - 1) / to) * to
}

#[inline(always)]
/// Rounds down a value to the nearest multiple of `to`.
///
/// # Arguments
///
/// * `value` - The value to be rounded down.
/// * `to` - The multiple to round down to.
///
/// # Returns
///
/// The rounded down value.
pub fn round_down(value: usize, to: usize) -> usize {
    value & !(to - 1)
}

#[inline(always)]
/// Checks if one range is completely contained within another range.
///
/// # Arguments
///
/// * `base1` - The base address of the first range.
/// * `size1` - The size of the first range.
/// * `base2` - The base address of the second range.
/// * `size2` - The size of the second range.
///
/// # Returns
///
/// Returns `true` if the first range is completely contained within the second range,
/// otherwise returns `false`.
pub fn range_in_range(base1: usize, size1: usize, base2: usize, size2: usize) -> bool {
    (base1 >= base2) && ((base1 + size1) <= (base2 + size2))
}

#[inline(always)]
/// Checks if the given address is within the specified range.
///
/// # Arguments
///
/// * `addr` - The address to check.
/// * `base` - The base address of the range.
/// * `size` - The size of the range.
///
/// # Returns
///
/// Returns `true` if the address is within the range, `false` otherwise.
pub fn in_range(addr: usize, base: usize, size: usize) -> bool {
    range_in_range(addr, 0, base, size)
}

#[inline(always)]
/// Extracts a bit field from a given value.
///
/// # Arguments
///
/// * `bits` - The value from which to extract the bit field.
/// * `off` - The offset of the bit field within the value.
/// * `len` - The length of the bit field.
///
/// # Returns
///
/// The extracted bit field as an unsigned integer.
pub fn bit_extract(bits: usize, off: usize, len: usize) -> usize {
    (bits >> off) & ((1 << len) - 1)
}

#[inline(always)]
/// Retrieves the value of a specific bit in a given bit sequence.
///
/// # Arguments
///
/// * `bits` - The bit sequence.
/// * `off` - The offset of the bit to retrieve.
///
/// # Returns
///
/// The value of the specified bit (0 or 1).
pub fn bit_get(bits: usize, off: usize) -> usize {
    (bits >> off) & 1
}

#[inline(always)]
/// Sets a bit at the specified offset in a given bit pattern.
///
/// # Arguments
///
/// * `bits` - The original bit pattern.
/// * `off` - The offset at which to set the bit.
///
/// # Returns
///
/// The updated bit pattern with the bit set at the specified offset.
pub fn bit_set(bits: usize, off: usize) -> usize {
    bits | (1 << off)
}

/// Finds the nth occurrence of a specified bit value in a bitmap within a given range.
///
/// # Arguments
///
/// * `bitmap` - The bitmap to search within.
/// * `start` - The starting index of the range to search within the bitmap.
/// * `size` - The size of the range to search within the bitmap.
/// * `nth` - The number of the occurrence to find.
/// * `set` - Specifies whether to search for set bits (true) or unset bits (false).
///
/// # Returns
///
/// * `Some(usize)` - The index of the nth occurrence of the specified bit value.
/// * `None` - If the size of the bitmap is too large or if the nth occurrence is not found.
pub fn bitmap_find_nth(bitmap: usize, start: usize, size: usize, nth: usize, set: bool) -> Option<usize> {
    if size + start > 64 {
        info!("bitmap_find_nth: bitmap size is too large");
        return None;
    }
    let mut count = 0;
    let bit = if set { 1 } else { 0 };
    let end = start + size;

    for i in start..end {
        if bit_extract(bitmap, i, 1) == bit {
            count += 1;
            if count == nth {
                return Some(i);
            }
        }
    }

    None
}

/// Reads or writes a value from/to a memory address.
///
/// # Arguments
///
/// * `addr` - The memory address to read from or write to.
/// * `width` - The width of the value in bytes (1, 2, 4, or 8).
/// * `val` - The value to write (ignored if `read` is `true`).
/// * `read` - Specifies whether to perform a read operation (`true`) or a write operation (`false`).
///
/// # Panics
///
/// This function will panic if `width` is not 1, 2, 4, or 8.
///
/// # Safety
///
/// This function uses unsafe Rust code to read from or write to a memory address directly.
/// It should only be used in situations where the memory access is known to be safe and valid.

pub fn ptr_read_write(addr: usize, width: usize, val: usize, read: bool) -> usize {
    if read {
        if width == 1 {
            unsafe { ptr::read(addr as *const u8) as usize }
        } else if width == 2 {
            unsafe { ptr::read(addr as *const u16) as usize }
        } else if width == 4 {
            unsafe { ptr::read(addr as *const u32) as usize }
        } else if width == 8 {
            unsafe { ptr::read(addr as *const u64) as usize }
        } else {
            panic!("ptr_read_write: illegal read len {}", width);
        }
    } else {
        if width == 1 {
            unsafe {
                ptr::write(addr as *mut u8, val as u8);
            }
        } else if width == 2 {
            unsafe {
                ptr::write(addr as *mut u16, val as u16);
            }
        } else if width == 4 {
            unsafe {
                ptr::write(addr as *mut u32, val as u32);
            }
        } else if width == 8 {
            unsafe {
                ptr::write(addr as *mut u64, val as u64);
            }
        } else {
            panic!("ptr_read_write: illegal write len {}", width);
        }
        0
    }
}
