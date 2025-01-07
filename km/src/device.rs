#[allow(unused_imports)]
use alloc::format;

use crate::{
    logger::LogLevel,
    memory::{is_valid_user_memory, ke_read_virtual_memory, ke_write_virtual_memory},
    println,
    process::Process,
};
use core::{ffi::c_void, ptr::null_mut};
use shared::ipc::Request;
use wdk::nt_success;
use wdk_sys::{
    ntddk::{ProbeForRead, RtlCopyMemoryNonTemporal},
    NTSTATUS, PIRP, STATUS_ACCESS_VIOLATION, STATUS_BUFFER_ALL_ZEROS, STATUS_SUCCESS,
    STATUS_UNSUCCESSFUL, _IO_STACK_LOCATION,
};

struct IoctlBuffer {
    len: u32,
    buf: *mut c_void,
    p_stack_location: *mut _IO_STACK_LOCATION,
    p_irp: PIRP,
}

impl IoctlBuffer {
    fn new(p_stack_location: *mut _IO_STACK_LOCATION, p_irp: PIRP) -> Self {
        IoctlBuffer {
            len: 0,
            buf: null_mut(),
            p_stack_location,
            p_irp,
        }
    }

    fn get_buf_to_req(&mut self) -> Result<Request, NTSTATUS> {
        self.receive()?;

        let input_buffer =
            unsafe { core::slice::from_raw_parts(self.buf as *const u8, self.len as usize) };
        if input_buffer.is_empty() {
            println!(LogLevel::Error, "Error reading string passed to IOCTL");
            return Err(STATUS_UNSUCCESSFUL);
        }

        let input_buffer_ptr: *const [u8; size_of::<Request>()] =
            input_buffer.as_ptr() as *const [u8; size_of::<Request>()];

        let request = unsafe {
            core::mem::transmute::<[u8; size_of::<Request>()], Request>(*input_buffer_ptr)
        };

        Ok(request)
    }

    fn receive(&mut self) -> Result<(), NTSTATUS> {
        let input_len: u32 = unsafe {
            (*self.p_stack_location)
                .Parameters
                .DeviceIoControl
                .InputBufferLength
        };

        let input_buffer: *mut c_void = unsafe { (*self.p_irp).AssociatedIrp.SystemBuffer };
        if input_buffer.is_null() {
            println!("Input buffer is null.");
            return Err(STATUS_BUFFER_ALL_ZEROS);
        };

        self.len = input_len;
        self.buf = input_buffer;

        Ok(())
    }

    fn send_str(&self, input_str: &str) -> Result<(), NTSTATUS> {
        unsafe { (*self.p_irp).IoStatus.__bindgen_anon_1.Status = STATUS_SUCCESS };

        let response = input_str.as_bytes();
        let response_len = response.len();
        unsafe { (*self.p_irp).IoStatus.Information = response_len as u64 };

        println!(
            LogLevel::Info,
            "Sending a message back to user-land {:?}",
            core::str::from_utf8(response).unwrap()
        );

        // Copy the data now into the buffer to send back to user-land.
        // The driver should not write directly to the buffer pointed to by Irp->UserBuffer.
        unsafe {
            if !(*self.p_irp).AssociatedIrp.SystemBuffer.is_null() {
                RtlCopyMemoryNonTemporal(
                    (*self.p_irp).AssociatedIrp.SystemBuffer,
                    response as *const _ as *mut c_void,
                    response_len as u64,
                );
            } else {
                println!(
                    LogLevel::Error,
                    "Error handling IOCTL, SystemBuffer was null."
                );
                return Err(STATUS_UNSUCCESSFUL);
            }
        }

        Ok(())
    }
}

pub fn ioctl_handler_read(
    p_stack_location: *mut _IO_STACK_LOCATION,
    p_irp: PIRP,
) -> Result<(), NTSTATUS> {
    let mut ioctl_buffer = IoctlBuffer::new(p_stack_location, p_irp);

    let request = ioctl_buffer.get_buf_to_req()?;
    println!(LogLevel::Info, "Received Request: {:?}", request);

    let Request {
        process_id,
        address,
        buffer,
        size,
    } = request;

    if size == 0 {
        println!(
            LogLevel::Error,
            "Invalid size specified in IOCTL request: {}", size
        );
        return Err(STATUS_UNSUCCESSFUL);
    }

    let Process { process } = Process::by_id(process_id)?;

    println!(
        LogLevel::Info,
        "Resolved process with PID {} and PEPROCESS at {:?}", process_id, process
    );

    // Pre-checks before accessing unsafe memory
    if !is_valid_user_memory(address as _, size as _) {
        println!(
            LogLevel::Error,
            "Invalid memory range: Address = {:#x}, Size = {}", address as usize, size
        );
        return Err(STATUS_ACCESS_VIOLATION);
    }

    unsafe { ProbeForRead(address as _, size as _, 1) };

    let mut bytes_read: usize = 0;
    let status = unsafe {
        ke_read_virtual_memory(process, address as _, buffer as _, size, &mut bytes_read)
    };

    if !nt_success(status) {
        println!(
            LogLevel::Error,
            "Error copying VirtualMemory! Error: {:?}", status
        );
        return Err(STATUS_UNSUCCESSFUL);
    }

    println!(
        LogLevel::Info,
        "Read {} bytes from {:#x}", bytes_read, address as usize
    );

    ioctl_buffer.send_str(&format!(
        "Copied {} bytes from {:#x}!",
        bytes_read, address as usize
    ))?;

    Ok(())
}

pub fn ioctl_handler_write(
    p_stack_location: *mut _IO_STACK_LOCATION,
    p_irp: PIRP,
) -> Result<(), NTSTATUS> {
    let mut ioctl_buffer = IoctlBuffer::new(p_stack_location, p_irp);

    let request = ioctl_buffer.get_buf_to_req()?;
    println!(LogLevel::Info, "Received Request: {:?}", request);

    let Request {
        process_id,
        address,
        buffer,
        size,
    } = request;

    let Process { process } = Process::by_id(process_id)?;

    println!(
        LogLevel::Info,
        "Resolved process with PID {} and PEPROCESS at {:?}", process_id, process
    );

    // Pre-checks before accessing unsafe memory
    if !is_valid_user_memory(address as _, size as _) {
        println!(
            LogLevel::Error,
            "Invalid memory range: Address = {:#x}, Size = {}", address as usize, size
        );
        return Err(STATUS_ACCESS_VIOLATION);
    }

    // Ensure the address is valid and accessible
    unsafe { ProbeForRead(address as _, size as _, 1) };

    let mut bytes_written: usize = 0;
    let status = unsafe {
        ke_write_virtual_memory(process, buffer as _, address as _, size, &mut bytes_written)
    };

    println!(
        LogLevel::Info,
        "Wrote {} bytes to {:#x}", bytes_written, address as usize
    );

    if !nt_success(status) {
        println!(
            LogLevel::Error,
            "Error copying VirtualMemory! Error: {:?}", status
        );
        return Err(STATUS_UNSUCCESSFUL);
    }

    ioctl_buffer.send_str(&format!(
        "Copied {} bytes to {:#x}!",
        bytes_written, address as usize
    ))?;

    Ok(())
}
