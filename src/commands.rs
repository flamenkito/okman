use std::time::Duration;

use anyhow::Result;

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

pub fn cmd_list(dev: &OnlyKeyDevice) -> Result<()> {
    let labels = get_labels(dev)?;
    let configured: Vec<_> = labels.iter().filter(|s| !s.label.is_empty()).collect();

    if configured.is_empty() {
        println!("No configured slots.");
        return Ok(());
    }

    let max_label = configured
        .iter()
        .map(|s| s.label.len())
        .max()
        .unwrap_or(5)
        .max(5);

    println!("┌────────┬─{}─┐", "─".repeat(max_label));
    println!("│ {:6} │ {:max_label$} │", "Slot", "Label");
    println!("├────────┼─{}─┤", "─".repeat(max_label));
    for slot in &configured {
        println!("│ {:6} │ {:max_label$} │", slot.name, slot.label);
    }
    println!("└────────┴─{}─┘", "─".repeat(max_label));

    Ok(())
}

pub struct SetOptions {
    pub label: Option<String>,
    pub username: Option<String>,
    pub password: bool,
    pub generate: bool,
    pub enter_after_password: bool,
    pub no_enter_after_password: bool,
}

pub fn cmd_set(dev: &OnlyKeyDevice, slot: &str, opts: SetOptions) -> Result<()> {
    if opts.label.is_none()
        && opts.username.is_none()
        && !opts.password
        && !opts.generate
        && !opts.enter_after_password
        && !opts.no_enter_after_password
    {
        anyhow::bail!(
            "Nothing to set. Use --label, --username, --password, --generate, --enter-after-password, or --no-enter-after-password"
        );
    }

    let slot_id = protocol::parse_slot(slot)?;
    let slot_name = protocol::slot_name(slot_id);

    if let Some(ref l) = opts.label {
        let resp = set_slot_field(dev, slot_id, MessageField::Label, l)?;
        println!("Label set for slot {}. Device: {}", slot_name, resp);
    }

    if let Some(ref u) = opts.username {
        let resp = set_slot_field(dev, slot_id, MessageField::Username, u)?;
        println!("Username set for slot {}. Device: {}", slot_name, resp);
    }

    if opts.password {
        let pw = rpassword::prompt_password(format!("Enter password for slot {}: ", slot_name))?;

        if pw.is_empty() {
            anyhow::bail!("Password cannot be empty");
        }

        let resp = set_slot_field(dev, slot_id, MessageField::Password, &pw)?;
        println!("Password set for slot {}. Device: {}", slot_name, resp);
    }

    if opts.generate {
        let pw = crate::password::generate();
        let resp = set_slot_field(dev, slot_id, MessageField::Password, &pw)?;
        println!("Generated password: {}", pw);
        println!("Password set for slot {}. Device: {}", slot_name, resp);
    }

    if opts.enter_after_password {
        let resp = set_slot_field_raw(
            dev,
            slot_id,
            MessageField::NextKey2,
            &[protocol::KEY_RETURN],
        )?;
        println!(
            "Enter-after-password enabled for slot {}. Device: {}",
            slot_name, resp
        );
    }

    if opts.no_enter_after_password {
        let resp = set_slot_field_raw(dev, slot_id, MessageField::NextKey2, &[0])?;
        println!(
            "Enter-after-password disabled for slot {}. Device: {}",
            slot_name, resp
        );
    }

    Ok(())
}

pub fn cmd_wipe(dev: &OnlyKeyDevice, slot: &str) -> Result<()> {
    let slot_id = protocol::parse_slot(slot)?;
    let slot_name = protocol::slot_name(slot_id);

    let responses = wipe_slot(dev, slot_id)?;
    for r in &responses {
        println!("{}", r);
    }
    println!("Slot {} wiped.", slot_name);

    Ok(())
}
