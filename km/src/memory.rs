use crate::ffi::MmCopyVirtualMemory;

use core::ffi::c_void;
use wdk::nt_success;
use wdk_sys::{
    ntddk::IoGetCurrentProcess, NTSTATUS, PEPROCESS, STATUS_ACCESS_DENIED, STATUS_SUCCESS,
};

pub unsafe fn ke_read_virtual_memory(
    process: PEPROCESS,
    source_address: *mut c_void,
    target_address: *mut c_void,
    size_t: u64,
    bytes_read: &mut u64,
) -> NTSTATUS {
    if nt_success(MmCopyVirtualMemory(
        process,
        source_address,
        IoGetCurrentProcess(),
        target_address,
        size_t,
        0,
        bytes_read,
    )) {
        STATUS_SUCCESS
    } else {
        STATUS_ACCESS_DENIED
    }
}

pub unsafe fn ke_write_virtual_memory(
    process: PEPROCESS,
    source_address: *mut c_void,
    target_address: *mut c_void,
    size_t: u64,
    bytes_read: &mut u64,
) -> NTSTATUS {
    if nt_success(MmCopyVirtualMemory(
        IoGetCurrentProcess(),
        source_address,
        process,
        target_address,
        size_t,
        0,
        bytes_read,
    )) {
        STATUS_SUCCESS
    } else {
        STATUS_ACCESS_DENIED
    }
}

pub fn is_valid_user_memory(address: usize, size: usize) -> bool {
    // Define user-mode memory bounds
    const USER_MODE_ADDRESS_LOWER_BOUND: usize = 0x00000000_00010000;
    const USER_MODE_ADDRESS_UPPER_BOUND: usize = 0x000007FF_FFFFFFFF;

    // Ensure address + size doesn't overflow
    if address.checked_add(size).is_none() {
        return false;
    }

    // Check if the address lies within valid user-mode memory
    address >= USER_MODE_ADDRESS_LOWER_BOUND && (address + size) <= USER_MODE_ADDRESS_UPPER_BOUND
}
