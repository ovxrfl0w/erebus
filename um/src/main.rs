#![deny(unreachable_pub)]
#![deny(missing_debug_implementations)]
#![deny(rust_2018_idioms)]
#![deny(bad_style)]
#![deny(unused)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::pedantic)]

mod driver;
mod utils;

use crate::{driver::Driver, utils::get_process_id, utils::str_to_address};
use shared::constants::DRIVER_UM_NAME;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return Err(format!("Usage: {} [address=0x12345678]", args[0]));
    }

    // Parse address
    let address = str_to_address(&args[1])?;

    // Get process ID by process name
    let process_name = "test-binary.exe"; // Could be a configurable input
    let process_id = get_process_id(process_name)
        .map_err(|err| format!("Failed to find process id for {process_name}! Error: {err}'"))?;

    // Open driver
    let driver = Driver::new(DRIVER_UM_NAME)
        .map_err(|_| format!("Failed to open driver device {DRIVER_UM_NAME}!"))?;

    // Read initial value from process memory
    let read_values = driver
        .read_process_memory::<i32>(process_id, address as *mut _, 1)
        .map_err(|err| format!("Could not read process memory! Error: {err}"))?;
    println!("Value at {address:#x} = {read_values:?}");

    // Write a value to process memory
    let write_values = [1337];
    driver
        .write_process_memory::<i32>(process_id, address as *mut _, &write_values)
        .map_err(|err| format!("Could not write process memory! Error: {err}"))?;
    println!("Finished writing, read again.");

    // Read the value again after writing
    let read_values = driver
        .read_process_memory::<i32>(process_id, address as *mut _, 1)
        .map_err(|err| format!("Could not read process memory! Error: {err}"))?;
    println!("Value at {address:#x} = {read_values:?}");

    Ok(())
}
