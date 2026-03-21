use crate::error::OnlyKeyError;

/// HID report size (excluding Windows leading 0x00).
pub const REPORT_SIZE: usize = 64;

/// Max payload bytes per single message: 64 - 4 (header) - 1 (msg) - 1 (slot/size) = 58
#[allow(dead_code)]
pub const MAX_PAYLOAD_SIZE: usize = 58;

/// Known OnlyKey USB Vendor/Product ID pairs.
pub const DEVICE_IDS: &[(u16, u16)] = &[(0x16C0, 0x0486), (0x1d50, 0x60FC)];

/// Message header bytes.
pub const MESSAGE_HEADER: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];

/// Command message types sent to the OnlyKey.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
#[allow(dead_code)]
pub enum Message {
    SetPin = 0xE1,
    SetSdPin = 0xE2,
    SetPdPin = 0xE3,
    SetTime = 0xE4,
    GetLabels = 0xE5,
    SetSlot = 0xE6,
    WipeSlot = 0xE7,
}

/// Field identifiers for `SetSlot` messages.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
#[allow(dead_code)]
pub enum MessageField {
    Label = 1,
    Username = 2,
    NextKey2 = 3,
    Delay2 = 4,
    Password = 5,
    NextKey3 = 6,
    Delay3 = 7,
    TfaType = 8,
    TotpKey = 9,
    YubiAuth = 10,
    IdleTimeout = 11,
    WipeMode = 12,
    KeyTypeSpeed = 13,
    KeyLayout = 14,
    Url = 15,
    NextKey1 = 16,
    Delay1 = 17,
    NextKey4 = 18,
    NextKey5 = 19,
}

/// USB HID keyboard Return/Enter scan code used for NextKey fields.
pub const KEY_RETURN: u8 = 128;

/// Parse a slot name like "1a", "3b" etc. into the numeric slot_id used by the protocol.
///
/// OnlyKey (non-DUO) slot mapping:
/// - 1a..6a → slot_id 1..6  (short press)
/// - 1b..6b → slot_id 7..12 (long press)
pub fn parse_slot(s: &str) -> Result<u8, OnlyKeyError> {
    let s = s.trim().to_lowercase();
    match s.as_str() {
        "1a" => Ok(1),
        "2a" => Ok(2),
        "3a" => Ok(3),
        "4a" => Ok(4),
        "5a" => Ok(5),
        "6a" => Ok(6),
        "1b" => Ok(7),
        "2b" => Ok(8),
        "3b" => Ok(9),
        "4b" => Ok(10),
        "5b" => Ok(11),
        "6b" => Ok(12),
        _ => Err(OnlyKeyError::InvalidSlot(s)),
    }
}

/// Human-readable name for a slot_id.
pub fn slot_name(slot_id: u8) -> &'static str {
    match slot_id {
        1 => "1a",
        2 => "2a",
        3 => "3a",
        4 => "4a",
        5 => "5a",
        6 => "6a",
        7 => "1b",
        8 => "2b",
        9 => "3b",
        10 => "4b",
        11 => "5b",
        12 => "6b",
        _ => "unknown",
    }
}

/// Build a 64-byte HID message.
///
/// Format: [0xFF 0xFF 0xFF 0xFF] [msg_type] [slot_id?] [field_id?] [payload...] [0x00 padding]
pub fn build_message(
    msg: Message,
    slot_id: Option<u8>,
    field: Option<MessageField>,
    payload: &[u8],
) -> [u8; REPORT_SIZE] {
    let mut buf = [0u8; REPORT_SIZE];

    // Header
    buf[0..4].copy_from_slice(&MESSAGE_HEADER);

    let mut pos = 4;

    // Message type
    buf[pos] = msg as u8;
    pos += 1;

    // Optional slot ID
    if let Some(sid) = slot_id {
        buf[pos] = sid;
        pos += 1;
    }

    // Optional field ID
    if let Some(f) = field {
        buf[pos] = f as u8;
        pos += 1;
    }

    // Payload (truncate if too large)
    let copy_len = payload.len().min(REPORT_SIZE - pos);
    buf[pos..pos + copy_len].copy_from_slice(&payload[..copy_len]);

    buf
}
