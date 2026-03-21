use std::time::Duration;

use crate::device::OnlyKeyDevice;
use crate::error::OnlyKeyError;
use crate::protocol::{self, Message, MessageField};

pub struct SlotLabel {
    pub slot_id: u8,
    pub name: String,
    pub label: String,
}

pub fn get_labels(dev: &OnlyKeyDevice) -> Result<Vec<SlotLabel>, OnlyKeyError> {
    dev.send_message(Message::GetLabels, None, None, &[])?;
    std::thread::sleep(Duration::from_millis(500));

    let mut slots = Vec::new();

    for _ in 0..12 {
        let raw = dev.read_string(1000)?;
        let parts: Vec<&str> = raw.splitn(2, '|').collect();
        if parts.len() < 2 {
            continue;
        }

        let raw_id = parts[0].bytes().next().unwrap_or(0);
        let mut slot_number = raw_id;
        if slot_number >= 16 {
            slot_number -= 6;
        }

        if !(1..=12).contains(&slot_number) {
            continue;
        }

        let label = parts[1].replace('\u{00FF}', " ").trim().to_string();

        slots.push(SlotLabel {
            slot_id: slot_number,
            name: protocol::slot_name(slot_number).to_string(),
            label,
        });
    }

    slots.sort_by_key(|s| s.slot_id);
    Ok(slots)
}

pub fn set_slot_field(
    dev: &OnlyKeyDevice,
    slot_id: u8,
    field: MessageField,
    value: &str,
) -> Result<String, OnlyKeyError> {
    set_slot_field_raw(dev, slot_id, field, value.as_bytes())
}

pub fn set_slot_field_raw(
    dev: &OnlyKeyDevice,
    slot_id: u8,
    field: MessageField,
    payload: &[u8],
) -> Result<String, OnlyKeyError> {
    dev.send_message(Message::SetSlot, Some(slot_id), Some(field), payload)?;
    std::thread::sleep(Duration::from_millis(200));
    dev.check_response()
}

pub fn wipe_slot(dev: &OnlyKeyDevice, slot_id: u8) -> Result<Vec<String>, OnlyKeyError> {
    dev.send_message(Message::WipeSlot, Some(slot_id), None, &[])?;
    std::thread::sleep(Duration::from_millis(200));

    let mut responses = Vec::new();
    for _ in 0..8 {
        match dev.read_string(500) {
            Ok(s) if !s.is_empty() => responses.push(s),
            _ => break,
        }
    }
    Ok(responses)
}
