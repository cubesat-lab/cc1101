#![no_std]

extern crate embedded_hal as hal;

#[cfg(feature = "std")]
extern crate std;

use core::fmt::{self, Display, Formatter};
use hal::spi::SpiDevice;

#[macro_use]
pub mod lowlevel;
mod types;

use lowlevel::{access::*, convert::*, registers::*, types::*};
pub use lowlevel::{
    types::{MachineState, MachineStateError},
    FIFO_MAX_SIZE,
};
pub use types::*;

/// CC1101 errors.
#[derive(Debug)]
pub enum Error<SpiE> {
    /// The TX FIFO buffer underflowed, too large packet for configured packet length.
    TxUnderflow,
    /// The RX FIFO buffer overflowed, too small buffer for configured packet length.
    RxOverflow,
    /// Corrupt packet received with invalid CRC.
    CrcMismatch,
    /// Invalid state read from MARCSTATE register
    InvalidState(u8),
    /// User Input Error
    UserInputError(usize),
    /// Platform-dependent SPI-errors, such as IO errors.
    Spi(SpiE),
}

impl<SpiE> From<SpiE> for Error<SpiE> {
    fn from(e: SpiE) -> Self {
        Error::Spi(e)
    }
}

impl<SpiE: Display> Display for Error<SpiE> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::RxOverflow => write!(f, "RX FIFO buffer overflowed"),
            Self::CrcMismatch => write!(f, "CRC mismatch"),
            Self::Spi(e) => write!(f, "SPI error: {}", e),
            _ => panic!("TODO"),
        }
    }
}

#[cfg(feature = "std")]
impl<SpiE: Display + core::fmt::Debug> std::error::Error for Error<SpiE> {}

/// High level API for interacting with the CC1101 radio chip.
pub struct Cc1101<SPI>(lowlevel::Cc1101<SPI>);

impl<SPI, SpiE> Cc1101<SPI>
where
    SPI: SpiDevice<u8, Error = SpiE>,
{
    pub fn new(spi: SPI) -> Result<Self, Error<SpiE>> {
        Ok(Cc1101(lowlevel::Cc1101::new(spi)?))
    }

    /// Last Chip Status Byte
    pub fn get_chip_status(&mut self) -> Option<StatusByte> {
        self.0.status
    }

    pub fn command(&mut self, command: CommandStrobe) -> Result<(), Error<SpiE>> {
        let command_strobe = match command {
            CommandStrobe::ResetChip => Command::SRES,
            CommandStrobe::EnableAndCalFreqSynth => Command::SFSTXON,
            CommandStrobe::TurnOffXosc => Command::SXOFF,
            CommandStrobe::CalFreqSynthAndTurnOff => Command::SCAL,
            CommandStrobe::EnableRx => Command::SRX,
            CommandStrobe::EnableTx => Command::STX,
            CommandStrobe::ExitRxTx => Command::SIDLE,
            CommandStrobe::StartWakeOnRadio => Command::SWOR,
            CommandStrobe::EnterPowerDownMode => Command::SPWD,
            CommandStrobe::FlushRxFifoBuffer => Command::SFRX,
            CommandStrobe::FlushTxFifoBuffer => Command::SFTX,
            CommandStrobe::ResetRtcToEvent1 => Command::SWORRST,
            CommandStrobe::NoOperation => Command::SNOP,
        };
        Ok(self.0.write_cmd_strobe(command_strobe)?)
    }

    /// Sets the carrier frequency (in Hertz).
    pub fn set_frequency(&mut self, hz: u64) -> Result<(), Error<SpiE>> {
        let (freq0, freq1, freq2) = from_frequency(hz);
        self.0.write_register(Config::FREQ0, freq0)?;
        self.0.write_register(Config::FREQ1, freq1)?;
        self.0.write_register(Config::FREQ2, freq2)?;
        Ok(())
    }

    /// Sets the frequency synthesizer intermediate frequency (in Hertz).
    pub fn set_freq_if(&mut self, hz: u64) -> Result<(), Error<SpiE>> {
        self.0
            .write_register(Config::FSCTRL1, FSCTRL1::default().freq_if(from_freq_if(hz)).bits())?;
        Ok(())
    }

    /// Sets the target value for the averaged amplitude from the digital channel filter.
    pub fn set_target_amplitude(&mut self, target: TargetAmplitude) -> Result<(), Error<SpiE>> {
        self.0.modify_register(Config::AGCCTRL2, |r| {
            AGCCTRL2(r).modify().magn_target(target.into()).bits()
        })?;
        Ok(())
    }

    /// Sets the filter length (in FSK/MSK mode) or decision boundary (in OOK/ASK mode) for the AGC.
    pub fn set_filter_length(&mut self, filter_length: FilterLength) -> Result<(), Error<SpiE>> {
        self.0.modify_register(Config::AGCCTRL0, |r| {
            AGCCTRL0(r).modify().filter_length(filter_length.into()).bits()
        })?;
        Ok(())
    }

    /// Configures when to run automatic calibration.
    pub fn set_autocalibration(&mut self, autocal: AutoCalibration) -> Result<(), Error<SpiE>> {
        self.0.modify_register(Config::MCSM0, |r| {
            MCSM0(r).modify().fs_autocal(autocal.into()).bits()
        })?;
        Ok(())
    }

    pub fn set_deviation(&mut self, deviation: u64) -> Result<(), Error<SpiE>> {
        let (mantissa, exponent) = from_deviation(deviation);
        self.0.write_register(
            Config::DEVIATN,
            DEVIATN::default().deviation_m(mantissa).deviation_e(exponent).bits(),
        )?;
        Ok(())
    }

    /// Sets the data rate (in bits per second).
    pub fn set_data_rate(&mut self, baud: u64) -> Result<(), Error<SpiE>> {
        let (mantissa, exponent) = from_drate(baud);
        self.0
            .modify_register(Config::MDMCFG4, |r| MDMCFG4(r).modify().drate_e(exponent).bits())?;
        self.0.write_register(Config::MDMCFG3, MDMCFG3::default().drate_m(mantissa).bits())?;
        Ok(())
    }

    pub fn enable_fec(&mut self, enable: bool) -> Result<(), Error<SpiE>> {
        self.0.modify_register(Config::MDMCFG1, |r| {
            MDMCFG1(r).modify().fec_en(enable as u8).bits()
        })?;
        Ok(())
    }

    pub fn set_cca_mode(&mut self, cca_mode: CcaMode) -> Result<(), Error<SpiE>> {
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

    pub fn set_num_preamble(&mut self, num_preamble: NumPreambleBytes) -> Result<(), Error<SpiE>> {
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

    pub fn crc_enable(&mut self, enable: bool) -> Result<(), Error<SpiE>> {
        self.0.modify_register(Config::PKTCTRL0, |r| {
            PKTCTRL0(r).modify().crc_en(enable as u8).bits()
        })?;
        Ok(())
    }

    /// Sets the channel bandwidth (in Hertz).
    pub fn set_chanbw(&mut self, bandwidth: u64) -> Result<(), Error<SpiE>> {
        let (mantissa, exponent) = from_chanbw(bandwidth);
        self.0.modify_register(Config::MDMCFG4, |r| {
            MDMCFG4(r).modify().chanbw_m(mantissa).chanbw_e(exponent).bits()
        })?;
        Ok(())
    }

    pub fn get_hw_info(&mut self) -> Result<(u8, u8), Error<SpiE>> {
        let partnum = self.0.read_register(Status::PARTNUM)?;
        let version = self.0.read_register(Status::VERSION)?;
        Ok((partnum, version))
    }

    /// Received Signal Strength Indicator is an estimate of the signal power level in the chosen channel.
    pub fn get_rssi_dbm(&mut self) -> Result<i16, Error<SpiE>> {
        Ok(from_rssi_to_rssi_dbm(self.0.read_register(Status::RSSI)?))
    }

    /// The Link Quality Indicator metric of the current quality of the received signal.
    pub fn get_lqi(&mut self) -> Result<u8, Error<SpiE>> {
        let lqi = self.0.read_register(Status::LQI)?;
        Ok(lqi & !(1u8 << 7))
    }

    /// Configure the sync word to use, and at what level it should be verified.
    pub fn set_sync_mode(&mut self, sync_mode: SyncMode) -> Result<(), Error<SpiE>> {
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
    pub fn set_modulation(&mut self, format: Modulation) -> Result<(), Error<SpiE>> {
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
    pub fn set_address_filter(&mut self, filter: AddressFilter) -> Result<(), Error<SpiE>> {
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
    pub fn set_packet_length(&mut self, length: PacketLength) -> Result<(), Error<SpiE>> {
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

    /// Turn data whitening on / off.
    pub fn white_data(&mut self, enable: bool) -> Result<(), Error<SpiE>> {
        self.0.modify_register(Config::PKTCTRL0, |r| {
            PKTCTRL0(r).modify().white_data(enable as u8).bits()
        })?;
        Ok(())
    }

    pub fn read_tx_bytes(&mut self) -> Result<u8, Error<SpiE>> {
        let txbytes = TXBYTES(self.0.read_register(Status::TXBYTES)?);
        let num_txbytes: u8 = txbytes.num_txbytes();

        if txbytes.txfifo_underflow() != 0 {
            return Err(Error::TxUnderflow);
        }

        Ok(num_txbytes)
    }

    pub fn read_rx_bytes(&mut self) -> Result<u8, Error<SpiE>> {
        let rxbytes = RXBYTES(self.0.read_register(Status::RXBYTES)?);
        let num_rxbytes: u8 = rxbytes.num_rxbytes();

        if rxbytes.rxfifo_overflow() != 0 {
            return Err(Error::RxOverflow);
        }

        Ok(num_rxbytes)
    }

    /// Read the Machine State
    pub fn read_machine_state(&mut self) -> Result<MachineState, Error<SpiE>> {
        let marcstate = MARCSTATE(self.0.read_register(Status::MARCSTATE)?);

        match MachineState::from_value(marcstate.marc_state()) {
            Ok(state) => Ok(state),
            Err(e) => match e {
                MachineStateError::InvalidState(value) => Err(Error::InvalidState(value)),
            },
        }
    }

    /// Read data from FIFO
    pub fn read_data(&mut self, data: &mut [u8]) -> Result<(), Error<SpiE>> {
        if data.len() <= FIFO_MAX_SIZE.into() {
            self.0.access_fifo(Access::Read, data)?;
        } else {
            return Err(Error::UserInputError(data.len()));
        }
        Ok(())
    }

    /// Write data into FIFO
    pub fn write_data(&mut self, data: &mut [u8]) -> Result<(), Error<SpiE>> {
        self.0.access_fifo(Access::Write, data)?;
        Ok(())
    }

    fn await_machine_state(&mut self, target_state: MachineState) -> Result<(), Error<SpiE>> {
        loop {
            let machine_state = self.read_machine_state()?;
            if target_state == machine_state {
                break;
            }
        }
        Ok(())
    }

    /// Configure some default settings, to be removed in the future.
    #[cfg_attr(rustfmt, rustfmt_skip)]
    pub fn set_defaults(&mut self) -> Result<(), Error<SpiE>> {
        self.command(CommandStrobe::ResetChip)?;

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
            .fs_autocal(AutoCalibration::FromIdle.into()).bits()
        )?;

        self.0.write_register(Config::AGCCTRL2, AGCCTRL2::default()
            .max_lna_gain(0x04).bits()
        )?;

        Ok(())
    }

    /// Set radio in Receive/Transmit/Idle mode.
    pub fn set_radio_mode(&mut self, radio_mode: RadioMode) -> Result<(), Error<SpiE>> {
        let target = match radio_mode {
            RadioMode::Idle => {
                self.command(CommandStrobe::ExitRxTx)?;
                MachineState::IDLE
            }
            RadioMode::Sleep => {
                self.set_radio_mode(RadioMode::Idle)?;
                self.command(CommandStrobe::EnterPowerDownMode)?;
                MachineState::SLEEP
            }
            RadioMode::Calibrate => {
                self.set_radio_mode(RadioMode::Idle)?;
                self.command(CommandStrobe::CalFreqSynthAndTurnOff)?;
                MachineState::MANCAL
            }
            RadioMode::Transmit => {
                self.set_radio_mode(RadioMode::Idle)?;
                self.command(CommandStrobe::EnableTx)?;
                MachineState::TX
            }

            RadioMode::Receive => {
                self.set_radio_mode(RadioMode::Idle)?;
                self.command(CommandStrobe::EnableRx)?;
                MachineState::RX
            }
        };
        self.await_machine_state(target)
    }

    fn rx_bytes_available(&mut self) -> Result<u8, Error<SpiE>> {
        let mut last = 0;

        loop {
            let num_rxbytes = self.read_rx_bytes()?;

            if (num_rxbytes > 0) && (num_rxbytes == last) {
                break;
            }

            last = num_rxbytes;
        }
        Ok(last)
    }

    // Should also be able to configure MCSM1.RXOFF_MODE to declare what state
    // to enter after fully receiving a packet.
    // Possible targets: IDLE, FSTON, TX, RX
    pub fn receive(&mut self, addr: &mut u8, buf: &mut [u8]) -> Result<u8, Error<SpiE>> {
        match self.rx_bytes_available() {
            Ok(_nbytes) => {
                self.read_data(buf)?;
                let length = buf[0];
                *addr = buf[1];
                let lqi = self.0.read_register(Status::LQI)?;
                self.await_machine_state(MachineState::IDLE)?;
                self.command(CommandStrobe::FlushRxFifoBuffer)?;
                if (lqi >> 7) != 1 {
                    Err(Error::CrcMismatch)
                } else {
                    Ok(length)
                }
            }
            Err(err) => {
                self.command(CommandStrobe::FlushRxFifoBuffer)?;
                Err(err)
            }
        }
    }

    pub fn transmit(&mut self, addr: &mut u8, buf: &mut [u8]) -> Result<(), Error<SpiE>> {
        // Check if the Tx fifo is empty and handle the undeflow condition
        // stfx command strobe
        let tx_len: u8 = buf.len() as u8;

        buf[0] = tx_len - 1;
        buf[1] = *addr;
        self.write_data(buf)?;
        self.command(CommandStrobe::EnableTx)?;
        self.await_machine_state(MachineState::IDLE)?;
        self.command(CommandStrobe::FlushTxFifoBuffer)?;

        Ok(())
    }
}
