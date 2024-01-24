//! Low level unrestricted access to the CC1101 radio chip.
use hal::blocking::spi::{Transfer, Write};
use hal::digital::v2::OutputPin;

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

pub struct Cc1101<SPI, CS> {
    pub(crate) spi: SPI,
    pub(crate) cs: CS,
    pub status: StatusByte,
    // gdo0: GDO0,
    // gdo2: GDO2,
}

#[derive(Debug)]
pub enum Error<SpiE, GpioE> {
    Spi(SpiE),
    Gpio(GpioE),
}

impl<SPI, CS, SpiE, GpioE> Cc1101<SPI, CS>
where
    SPI: Transfer<u8, Error = SpiE> + Write<u8, Error = SpiE>,
    CS: OutputPin<Error = GpioE>,
{
    pub fn new(spi: SPI, cs: CS) -> Result<Self, Error<SpiE, GpioE>> {
        let cc1101 = Cc1101 {
            spi,
            cs,
            status: StatusByte::default(),
        };
        Ok(cc1101)
    }

    pub fn read_register<R>(&mut self, reg: R) -> Result<u8, Error<SpiE, GpioE>>
    where
        R: Into<Register>,
    {
        let reg_addr = reg.into().raddr(access::Mode::Single);
        let mut buffer = [reg_addr, BLANK_BYTE];

        self.cs.set_low().map_err(Error::Gpio)?;
        self.spi.transfer(&mut buffer).map_err(Error::Spi)?;
        self.cs.set_high().map_err(Error::Gpio)?;

        self.status = StatusByte::from(buffer[0]);
        Ok(buffer[1])
    }

    pub fn access_fifo(
        &mut self,
        access: access::Access,
        data: &mut [u8],
    ) -> Result<(), Error<SpiE, GpioE>> {
        let mut buffer = [MultiByte::FIFO.addr(access, access::Mode::Burst)];

        self.cs.set_low().map_err(Error::Gpio)?;
        self.spi.transfer(&mut buffer).map_err(Error::Spi)?;
        self.spi.transfer(data).map_err(Error::Spi)?;
        self.cs.set_high().map_err(Error::Gpio)?;

        self.status = StatusByte::from(buffer[0]);
        Ok(())
    }

    pub fn write_cmd_strobe(&mut self, cmd: Command) -> Result<(), Error<SpiE, GpioE>> {
        let cmd_addr = cmd.addr(access::Access::Write, access::Mode::Single);
        let mut buffer = [cmd_addr];

        self.cs.set_low().map_err(Error::Gpio)?;
        self.spi.transfer(&mut buffer).map_err(Error::Spi)?;
        self.cs.set_high().map_err(Error::Gpio)?;

        self.status = StatusByte::from(buffer[0]);
        Ok(())
    }

    pub fn write_register<R>(&mut self, reg: R, byte: u8) -> Result<(), Error<SpiE, GpioE>>
    where
        R: Into<Register>,
    {
        let reg_addr = reg.into().waddr(access::Mode::Single);
        let mut buffer = [reg_addr, byte];

        self.cs.set_low().map_err(Error::Gpio)?;
        self.spi.transfer(&mut buffer).map_err(Error::Spi)?;
        self.cs.set_high().map_err(Error::Gpio)?;

        self.status = StatusByte::from(buffer[0]);
        Ok(())
    }

    pub fn modify_register<R, F>(&mut self, reg: R, f: F) -> Result<(), Error<SpiE, GpioE>>
    where
        R: Into<Register> + Copy,
        F: FnOnce(u8) -> u8,
    {
        let r = self.read_register(reg)?;
        self.write_register(reg, f(r))?;

        Ok(())
    }
}
