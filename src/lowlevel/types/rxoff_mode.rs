/// Rx Off Mode.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum RxOffMode {
    /// Next state after finishing packet reception: IDLE
    Idle = 0,
    /// Next state after finishing packet reception: FSTXON
    FsTxOn = 1,
    /// Next state after finishing packet reception: TX
    Tx = 2,
    /// Next state after finishing packet reception: Stay in RX
    StayInRx = 3,
}

impl From<RxOffMode> for u8 {
    fn from(value: RxOffMode) -> Self {
        value as Self
    }
}
