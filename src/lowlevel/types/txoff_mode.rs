/// Tx Off Mode.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TxOffMode {
    /// Next state after finishing packet transmission: IDLE
    Idle = 0,
    /// Next state after finishing packet transmission: FSTXON
    FsTxOn = 1,
    /// Next state after finishing packet transmission: Stay in TX (start sending preamble)
    StayInTx = 2,
    /// Next state after finishing packet transmission: RX
    Rx = 3,
}

impl From<TxOffMode> for u8 {
    fn from(value: TxOffMode) -> Self {
        value as Self
    }
}
