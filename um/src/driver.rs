use shared::{
    ioctl::{EREBUS_IOCTL_READ, EREBUS_IOCTL_WRITE},
    ipc::Request,
};
use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE},
        Storage::FileSystem::{CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_MODE, OPEN_EXISTING},
        System::IO::DeviceIoControl,
    },
};

#[derive(Debug)]
pub(crate) struct Driver {
    pub handle: HANDLE,
}

impl Driver {
    pub(crate) fn new(device_name: &str) -> Result<Self, String> {
        let handle_result = unsafe {
            CreateFileW(
                &HSTRING::from(device_name),
                GENERIC_WRITE.0 | GENERIC_READ.0,
                FILE_SHARE_MODE(0),
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        };

        match handle_result {
            Ok(handle) if !handle.is_invalid() => Ok(Self { handle }),
            _ => Err("Could not open driver device!".to_string()),
        }
    }

    pub(crate) fn issue_ioctl(
        &self,
        ioctl_code: u32,
        request: Request,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        #[cfg(debug_assertions)]
        println!("Issuing IOCTL {ioctl_code:#x}");

        let mut output_buffer = Vec::<u8>::with_capacity(1024);
        let mut bytes_returned: u32 = 0;

        #[allow(clippy::cast_possible_truncation)]
        unsafe {
            let result = DeviceIoControl(
                self.handle,
                ioctl_code,
                Some(std::ptr::addr_of!(request).cast()),
                size_of::<Request>() as _,
                Some(output_buffer.as_mut_ptr().cast()),
                output_buffer.capacity() as _,
                Some(&mut bytes_returned),
                None,
            );

            if result.is_err() {
                return Err("DeviceIoControl returned an error!".into());
            }

            output_buffer.set_len(bytes_returned as usize);
        }

        #[cfg(debug_assertions)]
        println!(
            "Sent IOCTL {ioctl_code:#x}. Kernel response: {:?}",
            &output_buffer[..bytes_returned as usize]
        );

        Ok(output_buffer)
    }

    pub(crate) fn read_process_memory<T>(
        &self,
        process_id: u64,
        address: *mut T,
        size: usize,
    ) -> Result<Vec<T>, Box<dyn std::error::Error>>
    where
        T: Copy + Sized,
    {
        let mut buffer: Vec<T> = Vec::with_capacity(size);

        let request = Request {
            process_id,
            address: address.cast(),
            buffer: buffer.as_mut_ptr().cast(),
            size: size * size_of::<T>(),
        };

        self.issue_ioctl(EREBUS_IOCTL_READ, request)?;

        unsafe { buffer.set_len(size) };

        Ok(buffer)
    }

    pub(crate) fn write_process_memory<T>(
        &self,
        process_id: u64,
        address: *mut std::ffi::c_void,
        buffer: &[T],
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        T: Copy + Sized,
    {
        let request = Request {
            process_id,
            address: address.cast(),
            buffer: buffer.as_ptr() as _,
            size: size_of_val(buffer),
        };

        self.issue_ioctl(EREBUS_IOCTL_WRITE, request)?;

        Ok(())
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle).ok() };
    }
}
