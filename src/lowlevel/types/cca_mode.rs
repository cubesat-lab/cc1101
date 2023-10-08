/// CCA Mode Configuration
#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub enum CcaModeConfig {
    /// Clear Channel Always
    ALWAYS = 0x00,
    /// Clear Channel If RSSI Below Threshold
    RSSI_BELOW_THR = 0x01,
    /// Clear Channel If RSSI Below Threshold Unless Receiving Packet
    RCV_PACKET = 0x02,
    /// Clear Channel If RSSI Below Threshold Unless Receiving Packet
    RSSI_BELOW_THR_UNLESS_RCV_PACKET = 0x03,
}

impl CcaModeConfig {
    pub fn value(&self) -> u8 {
        *self as u8
    }
}
