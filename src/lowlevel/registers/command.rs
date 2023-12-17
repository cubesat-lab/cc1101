#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]

pub enum Command {
    /// Reset chip
    SRES = 0x30,
    /// Enable/calibrate freq synthesizer
    SFSTXON = 0x31,
    /// Turn off crystal oscillator.
    SXOFF = 0x32,
    /// Calibrate freq synthesizer & disable
    SCAL = 0x33,
    /// Enable RX.
    SRX = 0x34,
    /// Enable TX.
    STX = 0x35,
    /// Exit RX / TX
    SIDLE = 0x36,
    /// AFC adjustment of freq synthesizer
    // SAFC = 0x37, // NOTE: This register was eliminated in the latest datasheet revision (SWRS061I - 2013.11.05)
    /// Start automatic RX polling sequence
    SWOR = 0x38,
    /// Enter pwr down mode when CSn goes hi
    SPWD = 0x39,
    /// Flush the RX FIFO buffer.
    SFRX = 0x3A,
    /// Flush the TX FIFO buffer.
    SFTX = 0x3B,
    /// Reset real time clock.
    SWORRST = 0x3C,
    /// No operation.
    SNOP = 0x3D,
}

impl Command {
    pub fn addr(
        &self,
        access: crate::lowlevel::access::Access,
        mode: crate::lowlevel::access::Mode,
    ) -> u8 {
        (access as u8) | (mode as u8) | (*self as u8)
    }
}

impl Into<crate::lowlevel::registers::Register> for Command {
    fn into(self) -> crate::lowlevel::registers::Register {
        crate::lowlevel::registers::Register::Command(self)
    }
}
