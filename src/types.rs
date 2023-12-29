/// Modulation format configuration.
pub enum Modulation {
    /// 2-FSK.
    BinaryFrequencyShiftKeying,
    /// GFSK.
    GaussianFrequencyShiftKeying,
    /// ASK / OOK.
    OnOffKeying,
    /// 4-FSK.
    FourFrequencyShiftKeying,
    /// MSK.
    MinimumShiftKeying,
}

/// Packet length configuration.
pub enum PacketLength {
    /// Set packet length to a fixed value.
    Fixed(u8),
    /// Set upper bound of variable packet length.
    Variable(u8),
    /// Infinite packet length, streaming mode.
    Infinite,
}

/// Number of preamble bytes to be transmitted.
pub enum NumPreambleBytes {
    // 2 preamble bytes
    Two,
    // 3 preamble bytes
    Three,
    // 4 preamble bytes
    Four,
    // 6 preamble bytes
    Six,
    // 8 preamble bytes
    Eight,
    // 12 preamble bytes
    Twelve,
    // 16 preamble bytes
    Sixteen,
    // 24 preamble bytes
    TwentyFour,
}

/// CCA mode configuration.
pub enum CcaMode {
    /// Always clear channel assessment.
    AlwaysClear,
    /// Clear channel assessment when RSSI is below threshold.
    ClearBelowThreshold,
    /// Clear channel assessment unless receiving packet.
    ClearWhenReceivingPacket,
    /// Clear channel assessment when RSSI is below threshold unless receiving packet.
    ClearBelowThresholdUnlessReceivingPacket,
}

/// Address check configuration.
pub enum AddressFilter {
    /// No address check.
    Disabled,
    /// Address check, no broadcast.
    Device(u8),
    /// Address check and 0 (0x00) broadcast.
    DeviceLowBroadcast(u8),
    /// Address check and 0 (0x00) and 255 (0xFF) broadcast.
    DeviceHighLowBroadcast(u8),
}

/// Radio operational mode.
pub enum RadioMode {
    Receive,
    Transmit,
    Idle,
}

/// Sync word configuration.
pub enum SyncMode {
    /// No sync word.
    Disabled,
    /// Match 15 of 16 bits of given sync word.
    MatchPartial(u16),
    /// Match 30 of 32 bits of a repetition of given sync word.
    MatchPartialRepeated(u16),
    /// Match 16 of 16 bits of given sync word.
    MatchFull(u16),
}
