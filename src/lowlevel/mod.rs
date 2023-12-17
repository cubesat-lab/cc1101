//! Low level unrestricted access to the CC1101 radio chip.
use hal::blocking::spi::{Transfer, Write};
use hal::digital::v2::OutputPin;

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

pub struct Cc1101<SPI, CS> {
    pub(crate) spi: SPI,
    pub(crate) cs: CS,
    //    gdo0: GDO0,
    //    gdo2: GDO2,
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

        let _status = buffer[0];
        let data = buffer[1];
        Ok(data)
    }

    pub fn read_fifo(
        &mut self,
        addr: &mut u8,
        len: &mut u8,
        buf: &mut [u8],
    ) -> Result<(), Error<SpiE, GpioE>> {
        // TODO Check this method
        let mut buffer = [
            MultiByte::FIFO.addr(access::Access::Read, access::Mode::Burst),
            BLANK_BYTE,
            BLANK_BYTE,
        ];

        self.cs.set_low().map_err(Error::Gpio)?;
        self.spi.transfer(&mut buffer).map_err(Error::Spi)?;
        self.spi.transfer(buf).map_err(Error::Spi)?;
        self.cs.set_high().map_err(Error::Gpio)?;

        let _status = buffer[0];
        *len = buffer[1];
        *addr = buffer[2];

        Ok(())
    }

    pub fn write_fifo(
        &mut self,
        addr: &mut u8,
        len: &mut u8,
        buf: &mut [u8],
    ) -> Result<(), Error<SpiE, GpioE>> {
        // TODO Check this method
        let mut buffer = [
            MultiByte::FIFO.addr(access::Access::Write, access::Mode::Burst),
            BLANK_BYTE,
            BLANK_BYTE,
        ];

        self.cs.set_low().map_err(Error::Gpio)?;
        self.spi.write(&mut buffer).map_err(Error::Spi)?;
        self.spi.write(buf).map_err(Error::Spi)?;
        self.cs.set_high().map_err(Error::Gpio)?;

        // TODO to be checked
        *len = buffer[1];
        *addr = buffer[2];

        Ok(())
    }

    pub fn write_cmd_strobe(&mut self, cmd: Command) -> Result<(), Error<SpiE, GpioE>> {
        let cmd_addr = cmd.addr(access::Access::Write, access::Mode::Single);

        self.cs.set_low().map_err(Error::Gpio)?;
        self.spi.write(&[cmd_addr]).map_err(Error::Spi)?;
        self.cs.set_high().map_err(Error::Gpio)?;

        Ok(())
    }

    pub fn write_register<R>(&mut self, reg: R, byte: u8) -> Result<(), Error<SpiE, GpioE>>
    where
        R: Into<Register>,
    {
        let reg_addr = reg.into().waddr(access::Mode::Single);

        self.cs.set_low().map_err(Error::Gpio)?;
        self.spi.write(&mut [reg_addr, byte]).map_err(Error::Spi)?;
        self.cs.set_high().map_err(Error::Gpio)?;

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
