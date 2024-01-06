#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub enum MultiByte {
    /// Power Amplifier Table
    PATABLE = 0x3E,
    /// FIFO Access
    FIFO = 0x3F,
}

impl MultiByte {
    pub fn addr(
        &self,
        access: crate::lowlevel::access::Access,
        mode: crate::lowlevel::access::Mode,
    ) -> u8 {
        (access as u8) | (mode as u8) | (*self as u8)
    }
}

impl Into<crate::lowlevel::registers::Register> for MultiByte {
    fn into(self) -> crate::lowlevel::registers::Register {
        crate::lowlevel::registers::Register::MultiByte(self)
    }
}
