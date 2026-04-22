use std::borrow::Cow;

use crate::device::DeviceType;
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
    SetPriv = 0xEF,
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
    BackupMode = 20,
    DerivedChallengeMode = 21,
    StoredChallengeMode = 22,
    SecProfileMode = 23,
    LedBrightness = 24,
    LockButton = 25,
    HmacMode = 26,
    SysadminMode = 27,
    TouchSense = 28,
}

/// OnlyKey "additional character" code for Return/Enter in NextKey fields.
/// Protocol values: 0 = none, 1 = Tab, 2 = Return.
pub const KEY_RETURN: u8 = 2;

const DUO_PIN_BLOCK_SIZE: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Green,
    Blue,
    Yellow,
    Purple,
}

impl Profile {
    pub fn offset(self) -> u8 {
        match self {
            Profile::Green => 0,
            Profile::Blue => 6,
            Profile::Yellow => 12,
            Profile::Purple => 18,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Profile::Green => "green",
            Profile::Blue => "blue",
            Profile::Yellow => "yellow",
            Profile::Purple => "purple",
        }
    }

    fn from_index(idx: u8) -> Option<Self> {
        match idx {
            0 => Some(Profile::Green),
            1 => Some(Profile::Blue),
            2 => Some(Profile::Yellow),
            3 => Some(Profile::Purple),
            _ => None,
        }
    }
}

pub fn slot_count(device_type: DeviceType) -> u8 {
    match device_type {
        DeviceType::Classic => 12,
        DeviceType::Duo => 24,
    }
}

pub fn parse_slot(
    s: &str,
    device_type: DeviceType,
    profile: Option<Profile>,
) -> Result<u8, OnlyKeyError> {
    match device_type {
        DeviceType::Classic => parse_slot_classic(s),
        DeviceType::Duo => parse_slot_duo(s, profile.unwrap_or(Profile::Green)),
    }
}

fn parse_slot_classic(s: &str) -> Result<u8, OnlyKeyError> {
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
        _ => Err(OnlyKeyError::InvalidSlot(format!(
            "'{s}'. Valid Classic slots: 1a-6a (short press) or 1b-6b (long press)"
        ))),
    }
}

/// DUO slot mapping (from OnlyKey App getSlotNum):
///   1a-3a → short press → N + profile_offset
///   1b-3b → long press  → N + 3 + profile_offset
fn parse_slot_duo(s: &str, profile: Profile) -> Result<u8, OnlyKeyError> {
    let s = s.trim().to_lowercase();
    let base = match s.as_str() {
        "1a" => 1,
        "2a" => 2,
        "3a" => 3,
        "1b" => 4,
        "2b" => 5,
        "3b" => 6,
        _ => {
            return Err(OnlyKeyError::InvalidSlot(format!(
                "'{s}'. Valid DUO slots: 1a-3a (short press) or 1b-3b (long press)"
            )));
        }
    };
    Ok(base + profile.offset())
}

pub fn slot_name(slot_id: u8, device_type: DeviceType) -> Cow<'static, str> {
    match device_type {
        DeviceType::Classic => Cow::Borrowed(slot_name_classic(slot_id)),
        DeviceType::Duo => Cow::Owned(slot_name_duo(slot_id)),
    }
}

fn slot_name_classic(slot_id: u8) -> &'static str {
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

fn slot_name_duo(slot_id: u8) -> String {
    if slot_id == 0 || slot_id > 24 {
        return "unknown".to_string();
    }
    let zero = slot_id - 1;
    let profile = match Profile::from_index(zero / 6) {
        Some(p) => p,
        None => return "unknown".to_string(),
    };
    let within = zero % 6;
    let button = (within % 3) + 1;
    let press = if within < 3 { 'a' } else { 'b' };
    format!("{} {}{}", profile.name(), button, press)
}

pub fn validate_pin(pin: &str) -> Result<(), OnlyKeyError> {
    if pin.len() < 7 || pin.len() > 10 {
        return Err(OnlyKeyError::InvalidPin("must be 7-10 digits".to_string()));
    }
    if !pin.chars().all(|c| ('1'..='6').contains(&c)) {
        return Err(OnlyKeyError::InvalidPin(
            "digits must be 1-6 only".to_string(),
        ));
    }
    Ok(())
}

pub fn encode_duo_pin(pin: &str) -> Vec<u8> {
    let mut buf = vec![0u8; DUO_PIN_BLOCK_SIZE];
    for (i, ch) in pin.chars().enumerate() {
        if i >= DUO_PIN_BLOCK_SIZE {
            break;
        }
        buf[i] = b'0' + ch.to_digit(10).unwrap_or(0) as u8;
    }
    buf
}

pub fn build_duo_init_payload(primary_pin: &str, sd_pin: &str) -> Vec<u8> {
    let mut payload = vec![0xFF];
    payload.extend(encode_duo_pin(primary_pin));
    payload.extend(encode_duo_pin(sd_pin));
    payload
}

pub fn encode_duo_unlock_payload(pin: &str) -> Vec<u8> {
    pin.chars()
        .map(|ch| b'0' + ch.to_digit(10).unwrap_or(0) as u8)
        .collect()
}

pub const BACKUP_KEY_SLOT: u8 = 131;
pub const BACKUP_KEY_TYPE: u8 = 161;

pub fn hash_backup_passphrase(passphrase: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(passphrase.as_bytes());
    hasher.finalize().to_vec()
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

#[cfg(test)]
mod tests {
    use super::*;

    const CLASSIC: DeviceType = DeviceType::Classic;
    const DUO: DeviceType = DeviceType::Duo;

    #[test]
    fn parse_slot_short_press() {
        assert_eq!(parse_slot("1a", CLASSIC, None).unwrap(), 1);
        assert_eq!(parse_slot("3a", CLASSIC, None).unwrap(), 3);
        assert_eq!(parse_slot("6a", CLASSIC, None).unwrap(), 6);
    }

    #[test]
    fn parse_slot_long_press() {
        assert_eq!(parse_slot("1b", CLASSIC, None).unwrap(), 7);
        assert_eq!(parse_slot("3b", CLASSIC, None).unwrap(), 9);
        assert_eq!(parse_slot("6b", CLASSIC, None).unwrap(), 12);
    }

    #[test]
    fn parse_slot_case_insensitive() {
        assert_eq!(parse_slot("1A", CLASSIC, None).unwrap(), 1);
        assert_eq!(parse_slot("6B", CLASSIC, None).unwrap(), 12);
    }

    #[test]
    fn parse_slot_trims_whitespace() {
        assert_eq!(parse_slot("  2a  ", CLASSIC, None).unwrap(), 2);
    }

    #[test]
    fn parse_slot_invalid() {
        assert!(parse_slot("7a", CLASSIC, None).is_err());
        assert!(parse_slot("0a", CLASSIC, None).is_err());
        assert!(parse_slot("1c", CLASSIC, None).is_err());
        assert!(parse_slot("", CLASSIC, None).is_err());
        assert!(parse_slot("abc", CLASSIC, None).is_err());
    }

    #[test]
    fn slot_name_classic_roundtrip() {
        for id in 1..=12 {
            let name = slot_name(id, CLASSIC);
            assert_eq!(parse_slot(&name, CLASSIC, None).unwrap(), id);
        }
    }

    #[test]
    fn slot_name_unknown() {
        assert_eq!(slot_name(0, CLASSIC), "unknown");
        assert_eq!(slot_name(13, CLASSIC), "unknown");
        assert_eq!(slot_name(255, CLASSIC), "unknown");
    }

    #[test]
    fn duo_parse_slot_green() {
        assert_eq!(parse_slot("1a", DUO, Some(Profile::Green)).unwrap(), 1);
        assert_eq!(parse_slot("2a", DUO, Some(Profile::Green)).unwrap(), 2);
        assert_eq!(parse_slot("3a", DUO, Some(Profile::Green)).unwrap(), 3);
        assert_eq!(parse_slot("1b", DUO, Some(Profile::Green)).unwrap(), 4);
        assert_eq!(parse_slot("2b", DUO, Some(Profile::Green)).unwrap(), 5);
        assert_eq!(parse_slot("3b", DUO, Some(Profile::Green)).unwrap(), 6);
    }

    #[test]
    fn duo_parse_slot_blue() {
        assert_eq!(parse_slot("1a", DUO, Some(Profile::Blue)).unwrap(), 7);
        assert_eq!(parse_slot("3b", DUO, Some(Profile::Blue)).unwrap(), 12);
    }

    #[test]
    fn duo_parse_slot_yellow() {
        assert_eq!(parse_slot("1a", DUO, Some(Profile::Yellow)).unwrap(), 13);
        assert_eq!(parse_slot("3b", DUO, Some(Profile::Yellow)).unwrap(), 18);
    }

    #[test]
    fn duo_parse_slot_purple() {
        assert_eq!(parse_slot("1a", DUO, Some(Profile::Purple)).unwrap(), 19);
        assert_eq!(parse_slot("3b", DUO, Some(Profile::Purple)).unwrap(), 24);
    }

    #[test]
    fn duo_parse_slot_invalid() {
        assert!(parse_slot("4a", DUO, Some(Profile::Green)).is_err());
        assert!(parse_slot("6b", DUO, Some(Profile::Green)).is_err());
    }

    #[test]
    fn duo_slot_name_green() {
        assert_eq!(slot_name(1, DUO), "green 1a");
        assert_eq!(slot_name(4, DUO), "green 1b");
        assert_eq!(slot_name(6, DUO), "green 3b");
    }

    #[test]
    fn duo_slot_name_purple() {
        assert_eq!(slot_name(19, DUO), "purple 1a");
        assert_eq!(slot_name(24, DUO), "purple 3b");
    }

    #[test]
    fn duo_slot_name_unknown() {
        assert_eq!(slot_name(0, DUO), "unknown");
        assert_eq!(slot_name(25, DUO), "unknown");
    }

    #[test]
    fn profile_offsets() {
        assert_eq!(Profile::Green.offset(), 0);
        assert_eq!(Profile::Blue.offset(), 6);
        assert_eq!(Profile::Yellow.offset(), 12);
        assert_eq!(Profile::Purple.offset(), 18);
    }

    #[test]
    fn slot_count_values() {
        assert_eq!(slot_count(CLASSIC), 12);
        assert_eq!(slot_count(DUO), 24);
    }

    #[test]
    fn validate_pin_valid() {
        assert!(validate_pin("1234561").is_ok());
        assert!(validate_pin("1111111111").is_ok());
        assert!(validate_pin("6543216").is_ok());
    }

    #[test]
    fn validate_pin_too_short() {
        assert!(validate_pin("123456").is_err());
        assert!(validate_pin("").is_err());
    }

    #[test]
    fn validate_pin_too_long() {
        assert!(validate_pin("12345611111").is_err());
    }

    #[test]
    fn validate_pin_invalid_digits() {
        assert!(validate_pin("1234567890").is_err());
        assert!(validate_pin("1234560").is_err());
        assert!(validate_pin("7777777").is_err());
    }

    #[test]
    fn encode_duo_pin_padded() {
        let encoded = encode_duo_pin("123");
        assert_eq!(encoded.len(), 16);
        assert_eq!(encoded[0], 49);
        assert_eq!(encoded[1], 50);
        assert_eq!(encoded[2], 51);
        assert!(encoded[3..].iter().all(|&b| b == 0));
    }

    #[test]
    fn encode_duo_pin_full() {
        let encoded = encode_duo_pin("1234561234");
        assert_eq!(encoded.len(), 16);
        assert_eq!(encoded[0], 49);
        assert_eq!(encoded[9], 52);
        assert!(encoded[10..].iter().all(|&b| b == 0));
    }

    #[test]
    fn build_duo_init_payload_format() {
        let payload = build_duo_init_payload("1234561", "6543211");
        assert_eq!(payload.len(), 33);
        assert_eq!(payload[0], 0xFF);
        assert_eq!(payload[1], 49);
        assert_eq!(payload[17], 54);
    }

    #[test]
    fn encode_duo_unlock_no_padding() {
        let payload = encode_duo_unlock_payload("123");
        assert_eq!(payload.len(), 3);
        assert_eq!(payload, vec![49, 50, 51]);
    }

    #[test]
    fn hash_backup_passphrase_is_32_bytes() {
        let hash = hash_backup_passphrase("abcdefghijklmnopqrstuvwxy");
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn hash_backup_passphrase_deterministic() {
        let h1 = hash_backup_passphrase("test passphrase for onlykey");
        let h2 = hash_backup_passphrase("test passphrase for onlykey");
        assert_eq!(h1, h2);
    }

    #[test]
    fn set_priv_message_format() {
        let mut payload = vec![BACKUP_KEY_TYPE];
        payload.extend(vec![0xAA; 32]);
        let msg = build_message(Message::SetPriv, Some(BACKUP_KEY_SLOT), None, &payload);
        assert_eq!(msg[4], 0xEF);
        assert_eq!(msg[5], 131);
        assert_eq!(msg[6], 161);
        assert_eq!(msg[7], 0xAA);
    }

    #[test]
    fn profile_from_index() {
        assert_eq!(Profile::from_index(0), Some(Profile::Green));
        assert_eq!(Profile::from_index(1), Some(Profile::Blue));
        assert_eq!(Profile::from_index(2), Some(Profile::Yellow));
        assert_eq!(Profile::from_index(3), Some(Profile::Purple));
        assert_eq!(Profile::from_index(4), None);
    }

    #[test]
    fn build_message_header() {
        let msg = build_message(Message::GetLabels, None, None, &[]);
        assert_eq!(&msg[0..4], &[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(msg[4], 0xE5);
        assert!(msg[5..].iter().all(|&b| b == 0));
    }

    #[test]
    fn build_message_set_slot_with_payload() {
        let msg = build_message(
            Message::SetSlot,
            Some(1),
            Some(MessageField::Password),
            b"hunter2",
        );
        assert_eq!(&msg[0..4], &[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(msg[4], 0xE6);
        assert_eq!(msg[5], 1);
        assert_eq!(msg[6], 5);
        assert_eq!(&msg[7..14], b"hunter2");
        assert!(msg[14..].iter().all(|&b| b == 0));
    }

    #[test]
    fn build_message_is_64_bytes() {
        let msg = build_message(Message::SetTime, None, None, &[0xAB; 100]);
        assert_eq!(msg.len(), REPORT_SIZE);
    }

    #[test]
    fn build_message_wipe_slot() {
        let msg = build_message(Message::WipeSlot, Some(7), None, &[]);
        assert_eq!(msg[4], 0xE7);
        assert_eq!(msg[5], 7);
        assert!(msg[6..].iter().all(|&b| b == 0));
    }

    #[test]
    fn build_message_set_pin() {
        let msg = build_message(Message::SetPin, None, None, &[]);
        assert_eq!(&msg[0..4], &MESSAGE_HEADER);
        assert_eq!(msg[4], 0xE1);
        assert!(msg[5..].iter().all(|&b| b == 0));
    }

    #[test]
    fn build_message_set_sd_pin() {
        let msg = build_message(Message::SetSdPin, None, None, &[]);
        assert_eq!(msg[4], 0xE2);
    }

    #[test]
    fn build_message_set_pd_pin() {
        let msg = build_message(Message::SetPdPin, None, None, &[]);
        assert_eq!(msg[4], 0xE3);
    }

    #[test]
    fn message_field_new_variants_repr() {
        assert_eq!(MessageField::BackupMode as u8, 20);
        assert_eq!(MessageField::DerivedChallengeMode as u8, 21);
        assert_eq!(MessageField::StoredChallengeMode as u8, 22);
        assert_eq!(MessageField::SecProfileMode as u8, 23);
        assert_eq!(MessageField::LedBrightness as u8, 24);
        assert_eq!(MessageField::LockButton as u8, 25);
        assert_eq!(MessageField::HmacMode as u8, 26);
        assert_eq!(MessageField::SysadminMode as u8, 27);
        assert_eq!(MessageField::TouchSense as u8, 28);
    }

    #[test]
    fn build_message_config_led_brightness() {
        let msg = build_message(
            Message::SetSlot,
            Some(1),
            Some(MessageField::LedBrightness),
            &[128],
        );
        assert_eq!(msg[4], 0xE6);
        assert_eq!(msg[5], 1);
        assert_eq!(msg[6], 24);
        assert_eq!(msg[7], 128);
        assert!(msg[8..].iter().all(|&b| b == 0));
    }

    #[test]
    fn build_message_config_type_speed_slot_zero() {
        let msg = build_message(
            Message::SetSlot,
            Some(0),
            Some(MessageField::KeyTypeSpeed),
            &[4],
        );
        assert_eq!(msg[4], 0xE6);
        assert_eq!(msg[5], 0);
        assert_eq!(msg[6], 13);
        assert_eq!(msg[7], 4);
    }
}
