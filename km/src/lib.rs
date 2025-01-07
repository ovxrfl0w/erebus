#![no_std]

extern crate alloc;
extern crate wdk_panic;

mod device;
mod ffi;
mod logger;
mod memory;
mod process;
mod utils;

use crate::{
    device::{ioctl_handler_read, ioctl_handler_write},
    ffi::IoGetCurrentIrpStackLocation,
    utils::{ToU16Vec, ToUnicodeString},
};

#[allow(unused_imports)]
use {
    crate::logger::{LogLevel, Logger},
    alloc::format,
};

use core::ptr::null_mut;
use shared::{
    constants::{DOS_DEVICE_NAME, NT_DEVICE_NAME},
    ioctl::{EREBUS_IOCTL_READ, EREBUS_IOCTL_WRITE},
};

use wdk::nt_success;
use wdk_alloc::WdkAllocator;
use wdk_sys::{
    ntddk::{
        IoCreateDevice, IoCreateSymbolicLink, IoDeleteDevice, IoDeleteSymbolicLink,
        IofCompleteRequest,
    },
    DEVICE_OBJECT, DO_BUFFERED_IO, DRIVER_OBJECT, FILE_DEVICE_SECURE_OPEN, FILE_DEVICE_UNKNOWN,
    IO_NO_INCREMENT, IRP_MJ_CLOSE, IRP_MJ_CREATE, IRP_MJ_DEVICE_CONTROL, NTSTATUS,
    PCUNICODE_STRING, PDEVICE_OBJECT, PDRIVER_OBJECT, PIRP, STATUS_INVALID_DEVICE_REQUEST,
    STATUS_SUCCESS, STATUS_UNSUCCESSFUL, _IO_STACK_LOCATION,
};

#[global_allocator]
static GLOBAL_ALLOCATOR: WdkAllocator = WdkAllocator;

#[export_name = "DriverEntry"]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "system" fn driver_entry(
    driver: PDRIVER_OBJECT,
    registry_path: PCUNICODE_STRING,
) -> NTSTATUS {
    println!(LogLevel::Success, "Driver loaded!");

    configure_driver(driver, registry_path)
}

#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn configure_driver(
    driver: *mut DRIVER_OBJECT,
    _registry_path: PCUNICODE_STRING,
) -> NTSTATUS {
    println!("Configuring driver...");

    let mut dos_name = DOS_DEVICE_NAME
        .to_unicode_string()
        .expect("unable to encode string to unicode.");

    let mut nt_name = NT_DEVICE_NAME
        .to_unicode_string()
        .expect("unable to encode string to unicode.");

    let mut device_object: PDEVICE_OBJECT = null_mut();

    let status = IoCreateDevice(
        driver,
        0,
        &mut nt_name,
        FILE_DEVICE_UNKNOWN,
        FILE_DEVICE_SECURE_OPEN,
        0,
        &mut device_object,
    );

    if !nt_success(status) {
        println!(
            LogLevel::Error,
            "Unable to create device via IoCreateDevice. Failed with code: {:x}.", status
        );
        return status;
    }

    let status = IoCreateSymbolicLink(&mut dos_name, &mut nt_name);
    if status != 0 {
        println!(
            LogLevel::Error,
            "Failed to create driver symbolic link. Error: {}", status
        );

        driver_exit(driver);
        return status;
    }

    (*driver).MajorFunction[IRP_MJ_CREATE as usize] = Some(create_close);
    (*driver).MajorFunction[IRP_MJ_CLOSE as usize] = Some(create_close);
    (*driver).MajorFunction[IRP_MJ_DEVICE_CONTROL as usize] = Some(handle_ioctl);

    (*driver).DriverUnload = Some(driver_exit);

    (*device_object).Flags |= DO_BUFFERED_IO;

    println!(LogLevel::Success, "Finished creating device!");

    STATUS_SUCCESS
}

extern "C" fn driver_exit(driver: *mut DRIVER_OBJECT) {
    let mut device_name = DOS_DEVICE_NAME
        .to_u16_vec()
        .to_unicode_string()
        .expect("unable to encode string to unicode.");
    let _ = unsafe { IoDeleteSymbolicLink(&mut device_name) };

    unsafe {
        IoDeleteDevice((*driver).DeviceObject);
    }

    println!("Driver exiting!");
}

#[allow(clippy::cast_possible_truncation)]
unsafe extern "C" fn create_close(_device: *mut DEVICE_OBJECT, p_irp: PIRP) -> NTSTATUS {
    (*p_irp).IoStatus.__bindgen_anon_1.Status = STATUS_SUCCESS;
    (*p_irp).IoStatus.Information = 0;

    IofCompleteRequest(p_irp, IO_NO_INCREMENT as i8);

    println!("IRP received...");

    STATUS_SUCCESS
}

macro_rules! handle_ioctl_fn {
    ($fn_name:ident, $p_stack_location:expr, $p_irp:expr) => {{
        if let Err(err) = $fn_name($p_stack_location, $p_irp) {
            println!(LogLevel::Error, "Error: {:#x}", err);
            err
        } else {
            STATUS_SUCCESS
        }
    }};
}

#[allow(clippy::cast_possible_truncation)]
unsafe extern "C" fn handle_ioctl(_device: *mut DEVICE_OBJECT, pirp: PIRP) -> NTSTATUS {
    let p_stack_location: *mut _IO_STACK_LOCATION = IoGetCurrentIrpStackLocation(pirp);

    if p_stack_location.is_null() {
        println!("Unable to get stack location for IRP.");
        return STATUS_UNSUCCESSFUL;
    }

    let control_code = (*p_stack_location).Parameters.DeviceIoControl.IoControlCode;
    println!(
        LogLevel::Info,
        "Received an IOCTL code: {:#x}", control_code
    );

    let status: NTSTATUS = match control_code {
        EREBUS_IOCTL_READ => {
            handle_ioctl_fn!(ioctl_handler_read, p_stack_location, pirp)
        }
        EREBUS_IOCTL_WRITE => {
            handle_ioctl_fn!(ioctl_handler_write, p_stack_location, pirp)
        }
        _ => {
            println!(
                LogLevel::Error,
                "Unhandled IOCTL control code: {:#x}", control_code
            );
            STATUS_INVALID_DEVICE_REQUEST
        }
    };

    IofCompleteRequest(pirp, IO_NO_INCREMENT as i8);

    status
}
