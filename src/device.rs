use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hidapi::{HidApi, HidDevice};

use crate::error::OnlyKeyError;
use crate::protocol::{self, Message, MessageField, DEVICE_IDS, REPORT_SIZE};

const READ_TIMEOUT_MS: i32 = 1000;
const CONNECT_RETRIES: u32 = 5;
const RETRY_DELAY: Duration = Duration::from_millis(1500);

const DUO_SERIAL: &str = "1000000000";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Classic,
    Duo,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::Classic => write!(f, "OnlyKey"),
            DeviceType::Duo => write!(f, "OnlyKey DUO"),
        }
    }
}

pub struct OnlyKeyDevice {
    hid: HidDevice,
    pub device_type: DeviceType,
}

impl OnlyKeyDevice {
    pub fn open() -> Result<Self, OnlyKeyError> {
        let api = HidApi::new()?;

        // macOS requires non-exclusive access to avoid privilege errors
        // when other apps (OnlyKey App, browser) also hold the device.
        #[cfg(target_os = "macos")]
        api.set_open_exclusive(false);

        let mut last_err = OnlyKeyError::DeviceNotFound;

        for attempt in 0..CONNECT_RETRIES {
            match Self::try_open(&api) {
                Ok(dev) => return Ok(dev),
                Err(e) => {
                    last_err = e;
                    if attempt < CONNECT_RETRIES - 1 {
                        std::thread::sleep(RETRY_DELAY);
                    }
                }
            }
        }

        Err(last_err)
    }

    fn try_open(api: &HidApi) -> Result<Self, OnlyKeyError> {
        for info in api.device_list() {
            let vid = info.vendor_id();
            let pid = info.product_id();

            if !DEVICE_IDS.iter().any(|&(v, p)| v == vid && p == pid) {
                continue;
            }

            let serial = info.serial_number().unwrap_or("");
            let usage_page = info.usage_page();
            let iface = info.interface_number();

            let is_duo = serial == DUO_SERIAL;
            let matches = if is_duo {
                usage_page == 0xFFAB || iface == 2
            } else {
                usage_page == 0xF1D0 || iface == 1
            };

            if matches {
                let hid = info.open_device(api)?;
                let device_type = if is_duo {
                    DeviceType::Duo
                } else {
                    DeviceType::Classic
                };
                return Ok(Self { hid, device_type });
            }
        }

        Err(OnlyKeyError::DeviceNotFound)
    }

    pub fn write(&self, data: &[u8; REPORT_SIZE]) -> Result<(), OnlyKeyError> {
        self.hid.write(data)?;
        Ok(())
    }

    pub fn read(&self, timeout_ms: i32) -> Result<Vec<u8>, OnlyKeyError> {
        let mut buf = [0u8; REPORT_SIZE];
        let n = self.hid.read_timeout(&mut buf, timeout_ms)?;
        Ok(buf[..n].to_vec())
    }

    pub fn read_string(&self, timeout_ms: i32) -> Result<String, OnlyKeyError> {
        let bytes = self.read(timeout_ms)?;
        let s: String = bytes
            .iter()
            .filter(|&&b| b != 0)
            .map(|&b| b as char)
            .collect();
        Ok(s)
    }

    pub fn send_message(
        &self,
        msg: Message,
        slot_id: Option<u8>,
        field: Option<MessageField>,
        payload: &[u8],
    ) -> Result<(), OnlyKeyError> {
        let buf = protocol::build_message(msg, slot_id, field, payload);
        self.write(&buf)
    }

    /// Send OKSETTIME and return the raw response string.
    /// Does not interpret the response — caller decides what to do with it.
    pub fn handshake_raw(&self) -> Result<String, OnlyKeyError> {
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;

        self.send_message(Message::SetTime, None, None, &epoch.to_be_bytes())?;
        std::thread::sleep(Duration::from_millis(500));

        self.read_string(READ_TIMEOUT_MS)
    }

    /// Send OKSETTIME and verify the device is unlocked.
    /// Returns the firmware version string on success.
    pub fn handshake(&self) -> Result<String, OnlyKeyError> {
        let response = self.handshake_raw()?;

        if response.contains("UNINITIALIZED") {
            return Err(OnlyKeyError::DeviceUninitialized);
        }
        if response.contains("INITIALIZED") {
            return Err(OnlyKeyError::DeviceLocked);
        }

        Ok(response)
    }

    pub fn check_response(&self) -> Result<String, OnlyKeyError> {
        let response = self.read_string(READ_TIMEOUT_MS)?;

        if response.contains("UNINITIALIZED") {
            return Err(OnlyKeyError::DeviceUninitialized);
        }
        if response.contains("INITIALIZED") {
            return Err(OnlyKeyError::DeviceLocked);
        }
        if response.contains("Error") {
            return Err(OnlyKeyError::DeviceMessage(response));
        }

        Ok(response)
    }
}
