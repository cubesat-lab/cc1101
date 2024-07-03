//! Low level unrestricted access to the CC1101 radio chip.

use hal::spi::{Operation, SpiDevice};

#[macro_use]
mod macros;
mod traits;

pub mod access;
pub mod convert;
pub mod registers;
pub mod types;

use self::registers::*;

pub const FXOSC: u64 = 26_000_000;
pub const FIFO_MAX_SIZE: u8 = 64;
const BLANK_BYTE: u8 = 0;

pub struct Cc1101<SPI> {
    pub(crate) spi: SPI,
    pub status: StatusByte,
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
            status: StatusByte::default(),
        };
        Ok(cc1101)
    }

    pub fn read_register<R>(&mut self, reg: R) -> Result<u8, SpiE>
    where
        R: Into<Register>,
    {
        let mut buffer = [reg.into().raddr(access::Mode::Single), BLANK_BYTE];

        self.spi.transfer_in_place(&mut buffer)?;

        self.status = StatusByte::from(buffer[0]);
        Ok(buffer[1])
    }

    pub fn access_fifo(&mut self, access: access::Access, data: &mut [u8]) -> Result<(), SpiE> {
        let mut buffer = [MultiByte::FIFO.addr(access, access::Mode::Burst)];

        self.spi.transaction(&mut [
            Operation::TransferInPlace(&mut buffer),
            Operation::TransferInPlace(data),
        ])?;

        self.status = StatusByte::from(buffer[0]);
        Ok(())
    }

    pub fn write_cmd_strobe(&mut self, cmd: Command) -> Result<(), SpiE> {
        let mut buffer = [cmd.addr(access::Access::Write, access::Mode::Single)];

        self.spi.transfer_in_place(&mut buffer)?;

        self.status = StatusByte::from(buffer[0]);
        Ok(())
    }

    pub fn write_register<R>(&mut self, reg: R, byte: u8) -> Result<(), SpiE>
    where
        R: Into<Register>,
    {
        let mut buffer = [reg.into().waddr(access::Mode::Single), byte];

        self.spi.transfer_in_place(&mut buffer)?;

        self.status = StatusByte::from(buffer[0]);
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
