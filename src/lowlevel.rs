//! Low level unrestricted access to the CC1101 radio chip.

use hal::spi::{Operation, SpiDevice};

#[macro_use]
mod macros;
mod access;
mod traits;

pub mod convert;
pub mod registers;
pub mod types;

use self::registers::*;

pub const FXOSC: u64 = 26_000_000;
const BLANK_BYTE: u8 = 0;

pub struct Cc1101<SPI> {
    pub(crate) spi: SPI,
    pub status: Option<StatusByte>,
    // gdo0: GDO0,
    // gdo2: GDO2,
}

impl<SPI, SpiE> Cc1101<SPI>
where
    SPI: SpiDevice<u8, Error = SpiE>,
{
    pub fn new(spi: SPI) -> Result<Self, SpiE> {
        let cc1101 = Cc1101 {
            spi,
            status: None,
        };
        Ok(cc1101)
    }

    pub fn read_register<R>(&mut self, reg: R) -> Result<u8, SpiE>
    where
        R: Into<Register>,
    {
        let mut buffer = [reg.into().raddr(access::Mode::Single), BLANK_BYTE];

        self.spi.transfer_in_place(&mut buffer)?;

        self.status = Some(StatusByte::from(buffer[0]));
        Ok(buffer[1])
    }

    pub fn read_fifo(&mut self, addr: &mut u8, len: &mut u8, buf: &mut [u8]) -> Result<(), SpiE> {
        let mut buffer = [
            MultiByte::FIFO.addr(access::Access::Read, access::Mode::Burst),
            BLANK_BYTE,
            BLANK_BYTE,
        ];

        self.spi.transaction(&mut [
            Operation::TransferInPlace(&mut buffer),
            Operation::TransferInPlace(buf),
        ])?;

        *len = buffer[1];
        *addr = buffer[2];

        self.status = Some(StatusByte::from(buffer[0]));
        Ok(())
    }

    pub fn access_fifo(&mut self, access: access::Access, data: &mut [u8]) -> Result<(), SpiE> {
        let mut buffer = [MultiByte::FIFO.addr(access, access::Mode::Burst)];

        self.spi.transaction(&mut [
            Operation::TransferInPlace(&mut buffer),
            Operation::TransferInPlace(data),
        ])?;

        self.status = Some(StatusByte::from(buffer[0]));
        Ok(())
    }

    pub fn write_cmd_strobe(&mut self, cmd: Command) -> Result<(), SpiE> {
        let mut buffer = [cmd.addr(access::Access::Write, access::Mode::Single)];

        self.spi.transfer_in_place(&mut buffer)?;

        if cmd == Command::SNOP {
            // SNOP is the only command with no effect and therefore can be used to get access to the chip status byte
            self.status = Some(StatusByte::from(buffer[0]));
        } else {
            // Discard returned chip status byte in `buffer[0]` as most probably, it will reflect the previous state
            // Set status to `None`, to inform the user about lack of valid state read
            self.status = None;
        }
        Ok(())
    }

    pub fn write_register<R>(&mut self, reg: R, byte: u8) -> Result<(), SpiE>
    where
        R: Into<Register>,
    {
        let mut buffer = [reg.into().waddr(access::Mode::Single), byte];

        self.spi.transfer_in_place(&mut buffer)?;

        self.status = Some(StatusByte::from(buffer[0]));
        Ok(())
    }

    pub fn modify_register<R, F>(&mut self, reg: R, f: F) -> Result<(), SpiE>
    where
        R: Into<Register> + Copy,
        F: FnOnce(u8) -> u8,
    {
        let r = self.read_register(reg)?;
        self.write_register(reg, f(r))?;

        Ok(())
    }
}
