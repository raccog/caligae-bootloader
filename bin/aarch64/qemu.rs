#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(panic_info_message)]

extern crate alloc;

use core::{
    arch::global_asm,
    cell::UnsafeCell,
    fmt::{self, Write},
};
use log::{self, info, LevelFilter, Log, Metadata, Record};

use caliga_bootloader::io::{io::Io, mmio::Mmio};

// The start procedure
global_asm!(include_str!("start.S"));

/// Address of UART0 on default QEMU for aarch64
pub const UART0_ADDR: usize = 0x0900_0000;

// An unimplemented allocator to see how it may be structured
//mod bump_allocator {
use core::alloc::{GlobalAlloc, Layout};

#[global_allocator]
static GLOBAL_ALLOCATOR: Aarch64QemuAlloc = Aarch64QemuAlloc {};

struct Aarch64QemuAlloc;

unsafe impl GlobalAlloc for Aarch64QemuAlloc {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        panic!("Allocation is unimplemented!");
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        unimplemented!();
    }
}
//}

#[repr(packed)]
pub struct Pl011Uart {
    data: Mmio<u8>,
}

impl Pl011Uart {
    /// Returns a [`Pl011Uart`] reference using a `base` address
    ///
    /// # Safety
    ///
    /// It is unsafe to use the referenced [`Pl011Uart`] because there could be an already existing reference.
    /// If multiple references to a single Uart exist, the owner of each reference could overwrite the registers
    /// used by the other reference.
    ///
    /// It should be ensured that when using this function, that another reference does not already exist.
    ///
    /// One exception to this rule is during a panic. As nothing else will be running, the panic handler
    /// is allowed to use this for re-initializing a Uart so the panic log can be somewhat reliably
    /// written to it. Note that this exception may not hold up if it's being used in multiple threads, as the
    /// threads might panic separately.
    pub unsafe fn new(base: usize) -> &'static mut Pl011Uart {
        &mut *(base as *mut Pl011Uart)
    }
}

impl Write for Pl011Uart {
    fn write_str(&mut self, out_string: &str) -> fmt::Result {
        for out_byte in out_string.bytes() {
            self.data.write(out_byte);
        }
        Ok(())
    }
}

/// A logger that outputs to a PL011 UART
///
/// This is a proof of concept to see what is necessary to set up a default logger.
///
/// # Interior Mutability
///
/// Internally, it uses an [`UnsafeCell`] to contain the UART struct because the method `log` would disallow
/// interior mutability, otherwise. Since this bootloader will always run on a single thread, there should be
/// no problems with race conditions.
struct UartPl011Logger {
    uart: UnsafeCell<&'static mut Pl011Uart>,
}

// Implement traits that are needed for `Log`
unsafe impl Sync for UartPl011Logger {}
unsafe impl Send for UartPl011Logger {}

impl Log for UartPl011Logger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level().to_level_filter() <= log::max_level()
    }

    // A very basic logger. Only outputs the log if it's possible without any allocations
    //
    // I want to move this into a cross-architecture implementation so that all logs can be formatted
    // the same. Also, it might be useful to use this in the panic logs, too.
    //
    // TODO: Deal with all the calls to `unwrap`
    fn log(&self, record: &Record<'_>) {
        // Get a mutable reference to the UART
        let uart = unsafe { &mut *self.uart.get() };

        // Write log level
        write!(uart, "[{}] ", record.level().as_str()).unwrap();

        // Try to write log without any allocations
        if let Some(args) = record.args().as_str() {
            uart.write_str(args).unwrap();
        } else {
            uart.write_str("Could not get log; allocator needed")
                .unwrap();
        }

        // Try to write log file and line without any allocations
        if let (Some(file_name), Some(line)) = (record.file(), record.line()) {
            write!(uart, ", {}:{:?}", file_name, line).unwrap();
        }

        uart.write_char('\n').unwrap();
    }

    fn flush(&self) {}
}

#[panic_handler]
fn handle_panic(info: &core::panic::PanicInfo) -> ! {
    // Re-initialize UART0 and print a panic log
    let uart = unsafe { Pl011Uart::new(UART0_ADDR) };
    // TODO: Maybe halt if this returns an error
    writeln!(uart, "[PANIC] {}", info).unwrap();
    loop {}
}

// The default logger
static mut LOGGER: Option<UartPl011Logger> = None;

#[no_mangle]
#[link_section = ".text.boot"]
pub unsafe extern "C" fn qemu_entry() {
    // Initialize UART0
    // The only other place it should be initialized is during a panic for emergency serial output
    let uart = unsafe { Pl011Uart::new(UART0_ADDR) };

    // Initialize logger using UART0
    let logger = {
        LOGGER = Some(UartPl011Logger { uart: uart.into() });
        LOGGER.as_ref().unwrap()
    };
    log::set_logger(logger).unwrap();
    log::set_max_level(LevelFilter::Debug);

    // Test out logger
    info!("Done with info log");

    // TODO: Run kernel
    panic!("End of bootloader reached");
}
