#![no_std]

extern crate embedded_hal as hal;

use hal::blocking::spi::{Transfer, Write};
use hal::digital::v2::OutputPin;

#[macro_use]
pub mod lowlevel;
mod types;

use lowlevel::{convert::*, registers::*, types::*};
pub use types::*;

/// CC1101 errors.
#[derive(Debug)]
pub enum Error<SpiE, GpioE> {
    /// The TX FIFO buffer underflowed, too large packet for configured packet length.
    TxUnderflow,
    /// The RX FIFO buffer overflowed, too small buffer for configured packet length.
    RxOverflow,
    /// Corrupt packet received with invalid CRC.
    CrcMismatch,
    /// Platform-dependent SPI-errors, such as IO errors.
    Spi(SpiE),
    /// Platform-dependent GPIO-errors, such as IO errors.
    Gpio(GpioE),
}

impl<SpiE, GpioE> From<lowlevel::Error<SpiE, GpioE>> for Error<SpiE, GpioE> {
    fn from(e: lowlevel::Error<SpiE, GpioE>) -> Self {
        match e {
            lowlevel::Error::Spi(inner) => Error::Spi(inner),
            lowlevel::Error::Gpio(inner) => Error::Gpio(inner),
        }
    }
}

/// High level API for interacting with the CC1101 radio chip.
pub struct Cc1101<SPI, CS>(lowlevel::Cc1101<SPI, CS>);

impl<SPI, CS, SpiE, GpioE> Cc1101<SPI, CS>
where
    SPI: Transfer<u8, Error = SpiE> + Write<u8, Error = SpiE>,
    CS: OutputPin<Error = GpioE>,
{
    pub fn new(spi: SPI, cs: CS) -> Result<Self, Error<SpiE, GpioE>> {
        Ok(Cc1101(lowlevel::Cc1101::new(spi, cs)?))
    }

    // Commands
    pub fn reset_chip(&mut self) -> Result<(), Error<SpiE, GpioE>> {
        Ok(self.0.write_cmd_strobe(Command::SRES)?)
    }

    pub fn enable_and_cal_freq_synth(&mut self) -> Result<(), Error<SpiE, GpioE>> {
        Ok(self.0.write_cmd_strobe(Command::SFSTXON)?)
    }

    pub fn disable_xosc(&mut self) -> Result<(), Error<SpiE, GpioE>> {
        Ok(self.0.write_cmd_strobe(Command::SXOFF)?)
    }

    pub fn cal_freq_synth_and_disable(&mut self) -> Result<(), Error<SpiE, GpioE>> {
        Ok(self.0.write_cmd_strobe(Command::SCAL)?)
    }

    pub fn enable_rx(&mut self) -> Result<(), Error<SpiE, GpioE>> {
        Ok(self.0.write_cmd_strobe(Command::SRX)?)
    }

    pub fn enable_tx(&mut self) -> Result<(), Error<SpiE, GpioE>> {
        Ok(self.0.write_cmd_strobe(Command::STX)?)
    }

    pub fn exit_rx_tx(&mut self) -> Result<(), Error<SpiE, GpioE>> {
        Ok(self.0.write_cmd_strobe(Command::SIDLE)?)
    }

    pub fn start_wake_on_radio(&mut self) -> Result<(), Error<SpiE, GpioE>> {
        Ok(self.0.write_cmd_strobe(Command::SWOR)?)
    }

    pub fn enter_power_down_mode(&mut self) -> Result<(), Error<SpiE, GpioE>> {
        Ok(self.0.write_cmd_strobe(Command::SPWD)?)
    }

    pub fn flush_rx_fifo(&mut self) -> Result<(), Error<SpiE, GpioE>> {
        Ok(self.0.write_cmd_strobe(Command::SFRX)?)
    }

    pub fn flush_tx_fifo(&mut self) -> Result<(), Error<SpiE, GpioE>> {
        Ok(self.0.write_cmd_strobe(Command::SFTX)?)
    }

    pub fn reset_rtc_to_event1(&mut self) -> Result<(), Error<SpiE, GpioE>> {
        Ok(self.0.write_cmd_strobe(Command::SWORRST)?)
    }

    pub fn nop(&mut self) -> Result<(), Error<SpiE, GpioE>> {
        Ok(self.0.write_cmd_strobe(Command::SNOP)?)
    }

    // Configurations
    pub fn set_frequency(&mut self, hz: u64) -> Result<(), Error<SpiE, GpioE>> {
        let (freq0, freq1, freq2) = from_frequency(hz);
        self.0.write_register(Config::FREQ0, freq0)?;
        self.0.write_register(Config::FREQ1, freq1)?;
        self.0.write_register(Config::FREQ2, freq2)?;
        Ok(())
    }

    pub fn set_deviation(&mut self, deviation: u64) -> Result<(), Error<SpiE, GpioE>> {
        let (mantissa, exponent) = from_deviation(deviation);
        self.0.write_register(
            Config::DEVIATN,
            DEVIATN::default().deviation_m(mantissa).deviation_e(exponent).bits(),
        )?;
        Ok(())
    }

    pub fn set_data_rate(&mut self, baud: u64) -> Result<(), Error<SpiE, GpioE>> {
        let (mantissa, exponent) = from_drate(baud);
        self.0
            .modify_register(Config::MDMCFG4, |r| MDMCFG4(r).modify().drate_e(exponent).bits())?;
        self.0.write_register(Config::MDMCFG3, MDMCFG3::default().drate_m(mantissa).bits())?;
        Ok(())
    }

    pub fn enable_fec(&mut self, enable: bool) -> Result<(), Error<SpiE, GpioE>> {
        self.0.modify_register(Config::MDMCFG1, |r| {
            MDMCFG1(r).modify().fec_en(enable as u8).bits()
        })?;
        Ok(())
    }

    pub fn set_cca_mode(&mut self, cca_mode: CcaMode) -> Result<(), Error<SpiE, GpioE>> {
        let mode = match cca_mode {
            CcaMode::AlwaysClear => CcaModeConfig::ALWAYS,
            CcaMode::ClearBelowThreshold => CcaModeConfig::RSSI_BELOW_THR,
            CcaMode::ClearWhenReceivingPacket => CcaModeConfig::RCV_PACKET,
            CcaMode::ClearBelowThresholdUnlessReceivingPacket => {
                CcaModeConfig::RSSI_BELOW_THR_UNLESS_RCV_PACKET
            }
        };

        self.0
            .modify_register(Config::MCSM1, |r| MCSM1(r).modify().cca_mode(mode.value()).bits())?;

        Ok(())
    }

    pub fn set_num_preamble(
        &mut self,
        num_preamble: NumPreambleBytes,
    ) -> Result<(), Error<SpiE, GpioE>> {
        let preamble_setting = match num_preamble {
            NumPreambleBytes::Two => NumPreamble::N_2,
            NumPreambleBytes::Three => NumPreamble::N_3,
            NumPreambleBytes::Four => NumPreamble::N_4,
            NumPreambleBytes::Six => NumPreamble::N_6,
            NumPreambleBytes::Eight => NumPreamble::N_8,
            NumPreambleBytes::Twelve => NumPreamble::N_12,
            NumPreambleBytes::Sixteen => NumPreamble::N_16,
            NumPreambleBytes::TwentyFour => NumPreamble::N_24,
        };

        self.0.write_register(
            Config::MDMCFG1,
            MDMCFG1::default().num_preamble(preamble_setting as u8).bits(),
        )?;
        Ok(())
    }

    pub fn crc_enable(&mut self, enable: bool) -> Result<(), Error<SpiE, GpioE>> {
        self.0.modify_register(Config::PKTCTRL0, |r| {
            PKTCTRL0(r).modify().crc_en(enable as u8).bits()
        })?;
        Ok(())
    }

    pub fn set_chanbw(&mut self, bandwidth: u64) -> Result<(), Error<SpiE, GpioE>> {
        let (mantissa, exponent) = from_chanbw(bandwidth);
        self.0.modify_register(Config::MDMCFG4, |r| {
            MDMCFG4(r).modify().chanbw_m(mantissa).chanbw_e(exponent).bits()
        })?;
        Ok(())
    }

    pub fn get_hw_info(&mut self) -> Result<(u8, u8), Error<SpiE, GpioE>> {
        let partnum = self.0.read_register(Status::PARTNUM)?;
        let version = self.0.read_register(Status::VERSION)?;
        Ok((partnum, version))
    }

    /// Received Signal Strength Indicator is an estimate of the signal power level in the chosen channel.
    pub fn get_rssi_dbm(&mut self) -> Result<i16, Error<SpiE, GpioE>> {
        Ok(from_rssi_to_rssi_dbm(self.0.read_register(Status::RSSI)?))
    }

    /// The Link Quality Indicator metric of the current quality of the received signal.
    pub fn get_lqi(&mut self) -> Result<u8, Error<SpiE, GpioE>> {
        let lqi = self.0.read_register(Status::LQI)?;
        Ok(lqi & !(1u8 << 7))
    }

    /// Configure the sync word to use, and at what level it should be verified.
    pub fn set_sync_mode(&mut self, sync_mode: SyncMode) -> Result<(), Error<SpiE, GpioE>> {
        let reset: u16 = (SYNC1::default().bits() as u16) << 8 | (SYNC0::default().bits() as u16);

        let (mode, word) = match sync_mode {
            SyncMode::Disabled => (SyncCheck::DISABLED, reset),
            SyncMode::MatchPartial(word) => (SyncCheck::CHECK_15_16, word),
            SyncMode::MatchPartialRepeated(word) => (SyncCheck::CHECK_30_32, word),
            SyncMode::MatchFull(word) => (SyncCheck::CHECK_16_16, word),
        };
        self.0.modify_register(Config::MDMCFG2, |r| {
            MDMCFG2(r).modify().sync_mode(mode.value()).bits()
        })?;
        self.0.write_register(Config::SYNC1, ((word >> 8) & 0xff) as u8)?;
        self.0.write_register(Config::SYNC0, (word & 0xff) as u8)?;
        Ok(())
    }

    /// Configure signal modulation.
    pub fn set_modulation(&mut self, format: Modulation) -> Result<(), Error<SpiE, GpioE>> {
        use lowlevel::types::ModFormat as MF;

        let value = match format {
            Modulation::BinaryFrequencyShiftKeying => MF::MOD_2FSK,
            Modulation::GaussianFrequencyShiftKeying => MF::MOD_GFSK,
            Modulation::OnOffKeying => MF::MOD_ASK_OOK,
            Modulation::FourFrequencyShiftKeying => MF::MOD_4FSK,
            Modulation::MinimumShiftKeying => MF::MOD_MSK,
        };
        self.0.modify_register(Config::MDMCFG2, |r| {
            MDMCFG2(r).modify().mod_format(value.value()).bits()
        })?;
        Ok(())
    }

    /// Configure device address, and address filtering.
    pub fn set_address_filter(&mut self, filter: AddressFilter) -> Result<(), Error<SpiE, GpioE>> {
        use lowlevel::types::AddressCheck as AC;

        let (mode, addr) = match filter {
            AddressFilter::Disabled => (AC::DISABLED, ADDR::default().bits()),
            AddressFilter::Device(addr) => (AC::SELF, addr),
            AddressFilter::DeviceLowBroadcast(addr) => (AC::SELF_LOW_BROADCAST, addr),
            AddressFilter::DeviceHighLowBroadcast(addr) => (AC::SELF_HIGH_LOW_BROADCAST, addr),
        };
        self.0.modify_register(Config::PKTCTRL1, |r| {
            PKTCTRL1(r).modify().adr_chk(mode.value()).bits()
        })?;
        self.0.write_register(Config::ADDR, addr)?;
        Ok(())
    }

    /// Configure packet mode, and length.
    pub fn set_packet_length(&mut self, length: PacketLength) -> Result<(), Error<SpiE, GpioE>> {
        use lowlevel::types::LengthConfig as LC;

        let (format, pktlen) = match length {
            PacketLength::Fixed(limit) => (LC::FIXED, limit),
            PacketLength::Variable(max_limit) => (LC::VARIABLE, max_limit),
            PacketLength::Infinite => (LC::INFINITE, PKTLEN::default().bits()),
        };
        self.0.modify_register(Config::PKTCTRL0, |r| {
            PKTCTRL0(r).modify().length_config(format.value()).bits()
        })?;
        self.0.write_register(Config::PKTLEN, pktlen)?;
        Ok(())
    }

    /// Set radio in Receive/Transmit/Idle mode.
    pub fn set_radio_mode(&mut self, radio_mode: RadioMode) -> Result<(), Error<SpiE, GpioE>> {
        let target = match radio_mode {
            RadioMode::Receive => {
                self.set_radio_mode(RadioMode::Idle)?;
                self.enable_rx()?;
                MachineState::RX
            }
            RadioMode::Transmit => {
                self.set_radio_mode(RadioMode::Idle)?;
                self.enable_tx()?;
                MachineState::TX
            }
            RadioMode::Idle => {
                self.exit_rx_tx()?;
                MachineState::IDLE
            }
        };
        self.await_machine_state(target)
    }

    /// Configure some default settings, to be removed in the future.
    #[cfg_attr(rustfmt, rustfmt_skip)]
    pub fn set_defaults(&mut self) -> Result<(), Error<SpiE, GpioE>> {
        self.reset_chip()?;

        self.0.write_register(Config::PKTCTRL0, PKTCTRL0::default()
            .white_data(0).bits()
        )?;

        self.0.write_register(Config::FSCTRL1, FSCTRL1::default()
            .freq_if(0x08).bits() // f_if = (f_osc / 2^10) * FREQ_IF
        )?;

        self.0.write_register(Config::MDMCFG2, MDMCFG2::default()
            .dem_dcfilt_off(1).bits()
        )?;

        self.0.write_register(Config::MCSM0, MCSM0::default()
            .fs_autocal(AutoCalibration::FROM_IDLE.value()).bits()
        )?;

        self.0.write_register(Config::AGCCTRL2, AGCCTRL2::default()
            .max_lna_gain(0x04).bits()
        )?;

        Ok(())
    }

    // Machine State
    pub fn read_machine_state(&mut self) -> Result<MachineState, Error<SpiE, GpioE>> {
        let marcstate = MARCSTATE(self.0.read_register(Status::MARCSTATE)?);
        Ok(MachineState::from_value(marcstate.marc_state()))
    }

    fn await_machine_state(
        &mut self,
        target_state: MachineState,
    ) -> Result<(), Error<SpiE, GpioE>> {
        loop {
            let machine_state = self.read_machine_state()?;
            if target_state == machine_state {
                break;
            }
        }
        Ok(())
    }

    fn rx_bytes_available(&mut self) -> Result<u8, Error<SpiE, GpioE>> {
        let mut last = 0;

        loop {
            let rxbytes = RXBYTES(self.0.read_register(Status::RXBYTES)?);
            if rxbytes.rxfifo_overflow() == 1 {
                return Err(Error::RxOverflow);
            }

            let nbytes = rxbytes.num_rxbytes();
            if (nbytes > 0) && (nbytes == last) {
                break;
            }

            last = nbytes;
        }
        Ok(last)
    }

    // Should also be able to configure MCSM1.RXOFF_MODE to declare what state
    // to enter after fully receiving a packet.
    // Possible targets: IDLE, FSTON, TX, RX
    pub fn receive(&mut self, addr: &mut u8, buf: &mut [u8]) -> Result<u8, Error<SpiE, GpioE>> {
        match self.rx_bytes_available() {
            Ok(_nbytes) => {
                let mut length = 0u8;
                self.0.read_fifo(addr, &mut length, buf)?;
                let lqi = self.0.read_register(Status::LQI)?;
                self.await_machine_state(MachineState::IDLE)?;
                self.flush_rx_fifo()?;

                // Go back to Rx mode
                // self.enable_rx()?;  // TODO: Check the logic

                if (lqi >> 7) != 1 {
                    Err(Error::CrcMismatch)
                } else {
                    Ok(length)
                }
            }
            Err(err) => {
                self.flush_rx_fifo()?;

                // Go back to Rx mode
                // self.enable_rx()?;  // TODO: Check the logic

                Err(err)
            }
        }
    }

    pub fn transmit(&mut self, addr: &mut u8, buf: &mut [u8]) -> Result<(), Error<SpiE, GpioE>> {
        // Check if the Tx fifo is empty and handle the undeflow condition
        // stfx command strobe
        let mut tx_len: u8 = buf.len() as u8;
        self.0.write_register(MultiByte::FIFO, tx_len)?;
        self.0.write_fifo(addr, &mut tx_len, buf)?;
        self.enable_tx()?;
        self.await_machine_state(MachineState::IDLE)?;
        self.flush_tx_fifo()?;
        Ok(())
    }
}
