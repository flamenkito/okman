use thiserror::Error;

#[derive(Debug, Error)]
pub enum OnlyKeyError {
    #[error("No OnlyKey device found. Is it plugged in?")]
    DeviceNotFound,

    #[error("OnlyKey is locked. Please enter your PIN on the device first.")]
    DeviceLocked,

    #[error("OnlyKey has no PIN set. Please set up your device first using the OnlyKey App.")]
    DeviceUninitialized,

    #[allow(dead_code)]
    #[error("Timed out waiting for OnlyKey response.")]
    Timeout,

    #[error("Invalid slot: {0}. Valid slots: 1a-6a (short press) or 1b-6b (long press)")]
    InvalidSlot(String),

    #[error("HID error: {0}")]
    Hid(#[from] hidapi::HidError),

    #[error("Device error: {0}")]
    DeviceMessage(String),
}
