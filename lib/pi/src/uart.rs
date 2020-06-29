use core::fmt;
use core::time::Duration;

use shim::io;
use shim::const_assert_size;

use volatile::prelude::*;
use volatile::{Volatile, ReadVolatile, Reserved};

use crate::timer;
use crate::common::IO_BASE;
use crate::gpio::{Gpio, Function};

/// The base address for the `MU` registers.
const MU_REG_BASE: usize = IO_BASE + 0x215040;

/// The `AUXENB` register from page 9 of the BCM2837 documentation.
const AUX_ENABLES: *mut Volatile<u8> = (IO_BASE + 0x215004) as *mut Volatile<u8>;

/// Enum representing bit fields of the `AUX_MU_LSR_REG` register.
#[repr(u8)]
enum LsrStatus {
    DataReady = 1,
    TxAvailable = 1 << 5,
}

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    IO: Volatile<u8>,
    IER: Volatile<u32>,
    IIR: Volatile<u32>,
    LCR: Volatile<u32>,
    MCR: Volatile<u32>,
    LSR: Volatile<u32>,
    MSR: ReadVolatile<u32>,
    SCRATCH: Volatile<u32>,
    CNTL: Volatile<u32>,
    STAT: ReadVolatile<u32>,
    BAUD: Volatile<u32>,
}

/// The Raspberry Pi's "mini UART".
pub struct MiniUart {
    registers: &'static mut Registers,
    timeout: Option<Duration>,
}

impl MiniUart {
    /// Initializes the mini UART by enabling it as an auxiliary peripheral,
    /// setting the data size to 8 bits, setting the BAUD rate to ~115200 (baud
    /// divider of 270), setting GPIO pins 14 and 15 to alternative function 5
    /// (TXD1/RDXD1), and finally enabling the UART transmitter and receiver.
    ///
    /// By default, reads will never time out. To set a read timeout, use
    /// `set_read_timeout()`.
    pub fn new() -> MiniUart {
        let registers = unsafe {
            // Enable the mini UART as an auxiliary device.
            (*AUX_ENABLES).or_mask(1);
            &mut *(MU_REG_BASE as *mut Registers)
        };

        registers.LCR.write(3);    // data size = 8 bits
        registers.BAUD.write(270);
        let gpio14 = Gpio::new(14);
        gpio14.into_alt(Function::Alt5);   //GPIO#14 set to Alt5 
        let gpio15 = Gpio::new(15);
        gpio15.into_alt(Function::Alt5);   //GPIO#15 set to Alt5
        registers.CNTL.write(3); // enable UART tx and rx
        MiniUart {
            registers : registers,
            timeout : None
        }
    }

    /// Set the read timeout to `t` duration.
    pub fn set_read_timeout(&mut self, t: Duration) {
        self.timeout = Some(t);
    }

    /// Write the byte `byte`. This method blocks until there is space available
    /// in the output FIFO.
    pub fn write_byte(&mut self, byte: u8) {
        while (self.registers.LSR.read() & (LsrStatus::TxAvailable as u32)) == 0 {
            continue;
        }
        self.registers.IO.write(byte);
    }

    /// Returns `true` if there is at least one byte ready to be read. If this
    /// method returns `true`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately. This method does not block.
    pub fn has_byte(&self) -> bool {
        (self.registers.LSR.read() & (LsrStatus::DataReady as u32)) != 0
    }

    /// Blocks until there is a byte ready to read. If a read timeout is set,
    /// this method blocks for at most that amount of time. Otherwise, this
    /// method blocks indefinitely until there is a byte to read.
    ///
    /// Returns `Ok(())` if a byte is ready to read. Returns `Err(())` if the
    /// timeout expired while waiting for a byte to be ready. If this method
    /// returns `Ok(())`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately.
    pub fn wait_for_byte(&self) -> Result<(), ()> {
        match self.timeout {
            Some(e) => {
                let ini = timer::current_time();
                while !self.has_byte() && (timer::current_time() < ini + e) {
                    continue;
                }
            }
            None => {
                while !self.has_byte()  {
                    continue;
                }
            }
        }
        if self.has_byte() {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Reads a byte. Blocks indefinitely until a byte is ready to be read.
    pub fn read_byte(&mut self) -> u8 {
        while !self.has_byte(){
            continue;
        }
        self.registers.IO.read()
    }
}

// Implement `fmt::Write` for `MiniUart`. A b'\r' byte should be written
// before writing any b'\n' byte.
impl fmt::Write for MiniUart {
    fn write_str(&mut self, s: &str) -> Result<(),core::fmt::Error> {
        for c in s.bytes() {
            if c == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(c);
        }
        return Ok(());
    }
}

mod uart_io {
    use super::io;
    use super::MiniUart;
    use volatile::prelude::*;

    // The `io::Read::read()` implementation must respect the read timeout by
    // waiting at most that time for the _first byte_. It should not wait for
    // any additional bytes but _should_ read as many bytes as possible. If the
    // read times out, an error of kind `TimedOut` should be returned.
    impl io::Read for MiniUart {
        fn read(&mut self, buf: &mut[u8]) -> Result<usize, io::Error> {
            let mut read_size : usize = 0;
            match self.wait_for_byte() {
                Ok(()) => {
                    buf[read_size] = self.read_byte();
                    read_size += 1;
                    while read_size < buf.len() {
                        while !self.has_byte() {
                            continue;
                        }
                        buf[read_size] = self.read_byte();
                        read_size += 1;
                    }
                    Ok(read_size)
                }
                Err(()) => Err(io::Error::new(io::ErrorKind::TimedOut, "first byte read timed out"))
            }
        }
    }

    // The `io::Write::write()` method must write all of the requested bytes
    // before returning.
    impl io::Write for MiniUart {
        fn write(&mut self, buf: &[u8]) -> Result<usize,io::Error> {
            let mut write_size :usize = 0;
            for b in buf.into_iter() {
                self.write_byte(b.clone());
                write_size += 1;
            }
            Ok(write_size)
        }

        fn flush(&mut self) -> Result<(),io::Error> {
            return Ok(());
        }
    }
}
