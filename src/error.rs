use thiserror::Error;

#[derive(Debug, Error)]
pub enum OnlyKeyError {
    #[error("No OnlyKey device found. Is it plugged in?")]
    DeviceNotFound,

    #[error("OnlyKey is locked")]
    DeviceLocked,

    #[error("OnlyKey has no PIN set")]
    DeviceUninitialized,

    #[allow(dead_code)]
    #[error("Timed out waiting for OnlyKey response.")]
    Timeout,

    #[error("Invalid slot: {0}")]
    InvalidSlot(String),

    #[error("HID error: {0}")]
    Hid(#[from] hidapi::HidError),

    #[error("Device error: {0}")]
    DeviceMessage(String),
}
