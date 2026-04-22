use std::time::Duration;

use anyhow::Result;

use crate::device::{DeviceType, OnlyKeyDevice};
use crate::error::OnlyKeyError;
use crate::protocol::{self, Message, MessageField, Profile};

pub struct SlotLabel {
    pub slot_id: u8,
    pub name: String,
    pub label: String,
}

pub fn get_labels(dev: &OnlyKeyDevice) -> Result<Vec<SlotLabel>, OnlyKeyError> {
    dev.send_message(Message::GetLabels, None, None, &[])?;
    std::thread::sleep(Duration::from_millis(500));

    let max_slots = protocol::slot_count(dev.device_type);
    let mut slots = Vec::new();

    for _ in 0..max_slots {
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

        let label = parts[1].replace('\u{00FF}', " ").trim().to_string();

        if !(1..=max_slots).contains(&slot_number) {
            continue;
        }

        slots.push(SlotLabel {
            slot_id: slot_number,
            name: protocol::slot_name(slot_number, dev.device_type).into_owned(),
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

pub fn cmd_list(dev: &OnlyKeyDevice, profile: Option<Profile>) -> Result<()> {
    let labels = get_labels(dev)?;

    let configured: Vec<_> = if dev.device_type == DeviceType::Duo {
        let prof = profile.unwrap_or(Profile::Green);
        let offset = prof.offset();
        labels
            .iter()
            .filter(|s| !s.label.is_empty())
            .filter(|s| s.slot_id > offset && s.slot_id <= offset + 6)
            .collect()
    } else {
        labels.iter().filter(|s| !s.label.is_empty()).collect()
    };

    if configured.is_empty() {
        println!("No configured slots.");
        return Ok(());
    }

    let max_slot = configured
        .iter()
        .map(|s| s.name.len())
        .max()
        .unwrap_or(4)
        .max(4);

    let max_label = configured
        .iter()
        .map(|s| s.label.len())
        .max()
        .unwrap_or(5)
        .max(5);

    println!("┌─{}─┬─{}─┐", "─".repeat(max_slot), "─".repeat(max_label));
    println!("│ {:max_slot$} │ {:max_label$} │", "Slot", "Label");
    println!("├─{}─┼─{}─┤", "─".repeat(max_slot), "─".repeat(max_label));
    for slot in &configured {
        println!("│ {:max_slot$} │ {:max_label$} │", slot.name, slot.label);
    }
    println!("└─{}─┴─{}─┘", "─".repeat(max_slot), "─".repeat(max_label));

    Ok(())
}

pub struct SetOptions {
    pub label: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub generate: bool,
    pub enter_after_password: bool,
    pub no_enter_after_password: bool,
}

fn requires_label(opts: &SetOptions) -> bool {
    (opts.password.is_some() || opts.generate) && opts.label.is_none()
}

pub fn cmd_set(
    dev: &OnlyKeyDevice,
    slot: &str,
    opts: SetOptions,
    profile: Option<Profile>,
) -> Result<()> {
    if opts.label.is_none()
        && opts.username.is_none()
        && opts.password.is_none()
        && !opts.generate
        && !opts.enter_after_password
        && !opts.no_enter_after_password
    {
        anyhow::bail!(
            "Nothing to set. Use --label, --username, --password, --generate, --enter-after-password, or --no-enter-after-password"
        );
    }

    if requires_label(&opts) {
        let labels = get_labels(dev)?;
        let slot_id = protocol::parse_slot(slot, dev.device_type, profile)?;
        let has_label = labels
            .iter()
            .any(|s| s.slot_id == slot_id && !s.label.is_empty());
        if !has_label {
            anyhow::bail!(
                "A --label is required when setting a password on a slot without one. \
                 The device only reports labels, so unlabeled slots are invisible to 'okman list'."
            );
        }
    }

    let slot_id = protocol::parse_slot(slot, dev.device_type, profile)?;
    let slot_name = protocol::slot_name(slot_id, dev.device_type);

    if let Some(ref l) = opts.label {
        let resp = set_slot_field(dev, slot_id, MessageField::Label, l)?;
        println!("Label set for slot {}. Device: {}", slot_name, resp);
    }

    if let Some(ref u) = opts.username {
        let resp = set_slot_field(dev, slot_id, MessageField::Username, u)?;
        println!("Username set for slot {}. Device: {}", slot_name, resp);
    }

    if let Some(ref pw_arg) = opts.password {
        let pw = if pw_arg.is_empty() {
            rpassword::prompt_password(format!("Enter password for slot {}: ", slot_name))?
        } else {
            pw_arg.clone()
        };

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

pub struct ConfigOptions {
    pub lock_timeout: Option<u8>,
    pub led_brightness: Option<u8>,
    pub keyboard_layout: Option<u8>,
    pub type_speed: Option<u8>,
    pub lock_button: Option<u8>,
    pub wipe_mode: Option<u8>,
    pub touch_sense: Option<u8>,
    pub sysadmin_mode: Option<u8>,
    pub backup_mode: Option<u8>,
    pub derived_challenge_mode: Option<u8>,
    pub stored_challenge_mode: Option<u8>,
    pub hmac_mode: Option<u8>,
    pub second_profile_mode: Option<u8>,
}

impl ConfigOptions {
    pub fn has_any(&self) -> bool {
        self.lock_timeout.is_some()
            || self.led_brightness.is_some()
            || self.keyboard_layout.is_some()
            || self.type_speed.is_some()
            || self.lock_button.is_some()
            || self.wipe_mode.is_some()
            || self.touch_sense.is_some()
            || self.sysadmin_mode.is_some()
            || self.backup_mode.is_some()
            || self.derived_challenge_mode.is_some()
            || self.stored_challenge_mode.is_some()
            || self.hmac_mode.is_some()
            || self.second_profile_mode.is_some()
    }
}

fn config_slot_id(field: MessageField) -> u8 {
    match field {
        MessageField::KeyTypeSpeed => 0,
        _ => 1,
    }
}

fn set_config_field(
    dev: &OnlyKeyDevice,
    field: MessageField,
    value: u8,
) -> Result<String, OnlyKeyError> {
    let slot_id = config_slot_id(field);
    set_slot_field_raw(dev, slot_id, field, &[value])
}

pub fn cmd_config(dev: &OnlyKeyDevice, opts: &ConfigOptions) -> Result<()> {
    if !opts.has_any() {
        anyhow::bail!(
            "Nothing to configure. Use --lock-timeout, --led-brightness, --keyboard-layout, \
             --type-speed, --lock-button, --wipe-mode, --touch-sense, --sysadmin-mode, \
             --backup-mode, --derived-challenge-mode, --stored-challenge-mode, --hmac-mode, \
             or --second-profile-mode"
        );
    }

    let fields: &[(Option<u8>, MessageField, &str)] = &[
        (opts.lock_timeout, MessageField::IdleTimeout, "Lock timeout"),
        (
            opts.led_brightness,
            MessageField::LedBrightness,
            "LED brightness",
        ),
        (
            opts.keyboard_layout,
            MessageField::KeyLayout,
            "Keyboard layout",
        ),
        (opts.type_speed, MessageField::KeyTypeSpeed, "Type speed"),
        (opts.lock_button, MessageField::LockButton, "Lock button"),
        (opts.wipe_mode, MessageField::WipeMode, "Wipe mode"),
        (
            opts.touch_sense,
            MessageField::TouchSense,
            "Touch sensitivity",
        ),
        (
            opts.sysadmin_mode,
            MessageField::SysadminMode,
            "Sysadmin mode",
        ),
        (opts.backup_mode, MessageField::BackupMode, "Backup mode"),
        (
            opts.derived_challenge_mode,
            MessageField::DerivedChallengeMode,
            "Derived challenge mode",
        ),
        (
            opts.stored_challenge_mode,
            MessageField::StoredChallengeMode,
            "Stored challenge mode",
        ),
        (opts.hmac_mode, MessageField::HmacMode, "HMAC mode"),
        (
            opts.second_profile_mode,
            MessageField::SecProfileMode,
            "Second profile mode",
        ),
    ];

    for (value, field, label) in fields {
        if let Some(v) = value {
            let resp = set_config_field(dev, *field, *v)?;
            println!("{} set to {}. Device: {}", label, v, resp);
        }
    }

    Ok(())
}

fn pin_setup_step(dev: &OnlyKeyDevice, msg: Message, label: &str) -> Result<()> {
    println!("\n=== {} ===", label);

    dev.send_message(msg, None, None, &[])?;
    std::thread::sleep(Duration::from_millis(500));
    let prompt1 = dev.read_string(2000)?;
    if !prompt1.is_empty() {
        println!("Device: {}", prompt1);
    }

    println!("Enter your PIN (7-10 digits) on the OnlyKey device.");
    eprint!("Press Enter here when done...");
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf)?;

    dev.send_message(msg, None, None, &[])?;
    std::thread::sleep(Duration::from_millis(500));
    let prompt2 = dev.read_string(2000)?;
    if !prompt2.is_empty() {
        println!("Device: {}", prompt2);
    }

    dev.send_message(msg, None, None, &[])?;
    std::thread::sleep(Duration::from_millis(500));
    let prompt3 = dev.read_string(2000)?;
    if !prompt3.is_empty() {
        println!("Device: {}", prompt3);
    }

    println!("Confirm your PIN on the OnlyKey device.");
    eprint!("Press Enter here when done...");
    buf.clear();
    std::io::stdin().read_line(&mut buf)?;

    dev.send_message(msg, None, None, &[])?;
    std::thread::sleep(Duration::from_millis(1500));
    let result = dev.read_string(2000)?;
    if !result.is_empty() {
        println!("Device: {}", result);
    }

    if result.contains("Error") {
        anyhow::bail!("{} setup failed: {}", label, result);
    }

    println!("{} set successfully.", label);
    Ok(())
}

pub fn cmd_init(dev: &OnlyKeyDevice) -> Result<()> {
    let response = dev.handshake_raw()?;

    let is_duo_no_pin = dev.device_type == DeviceType::Duo && response.ends_with('n');

    if is_duo_no_pin {
        anyhow::bail!(
            "Device is already initialized (no PIN, always unlocked). \
             Factory reset the device to re-initialize."
        );
    }

    if !response.contains("UNINITIALIZED") {
        if response.contains("UNLOCKED") {
            anyhow::bail!(
                "Device is already initialized and unlocked. \
                 Use the OnlyKey App to change PINs on an initialized device."
            );
        }
        if response.contains("INITIALIZED") {
            anyhow::bail!(
                "Device is already initialized (locked). \
                 Enter your PIN to unlock, or use the OnlyKey App to change PINs."
            );
        }
        anyhow::bail!("Unexpected device state: {}", response);
    }

    match dev.device_type {
        DeviceType::Classic => cmd_init_classic(dev),
        DeviceType::Duo => cmd_init_duo(dev),
    }
}

fn cmd_init_classic(dev: &OnlyKeyDevice) -> Result<()> {
    println!("Device is UNINITIALIZED. Starting PIN setup.");
    println!("PINs are entered on the physical OnlyKey device (7-10 digits).");

    pin_setup_step(dev, Message::SetPin, "Primary PIN")?;
    pin_setup_step(dev, Message::SetPdPin, "Second Profile PIN")?;
    pin_setup_step(dev, Message::SetSdPin, "Self-Destruct PIN")?;

    println!("\nDevice initialized successfully!");
    Ok(())
}

fn prompt_and_confirm_pin(label: &str) -> Result<String> {
    loop {
        let pin = rpassword::prompt_password(format!("{} (7-10 digits, 1-6 only): ", label))?;
        if let Err(e) = protocol::validate_pin(&pin) {
            eprintln!("{}", e);
            continue;
        }
        let confirm = rpassword::prompt_password(format!("Confirm {}: ", label))?;
        if pin != confirm {
            eprintln!("Error: PINs do not match. Try again.");
            continue;
        }
        return Ok(pin);
    }
}

fn cmd_init_duo(dev: &OnlyKeyDevice) -> Result<()> {
    println!("Device is UNINITIALIZED (OnlyKey DUO). Starting setup.\n");

    eprint!("Set a device PIN? (without PIN, device stays unlocked) [y/N]: ");
    let mut pin_answer = String::new();
    std::io::stdin().read_line(&mut pin_answer)?;

    if pin_answer.trim().to_lowercase() == "y" {
        println!("PINs must be 7-10 digits, using digits 1-6 only.\n");

        let primary = prompt_and_confirm_pin("Primary PIN")?;

        eprint!("Set a self-destruct PIN? [y/N]: ");
        let mut sd_answer = String::new();
        std::io::stdin().read_line(&mut sd_answer)?;
        let sd_pin = if sd_answer.trim().to_lowercase() == "y" {
            prompt_and_confirm_pin("Self-Destruct PIN")?
        } else {
            String::new()
        };

        let payload = protocol::build_duo_init_payload(&primary, &sd_pin);

        dev.send_message(Message::SetPin, None, None, &payload)?;
        std::thread::sleep(Duration::from_millis(1500));
        let result = dev.read_string(2000)?;
        if !result.is_empty() {
            println!("Device: {}", result);
        }

        if result.contains("Error") {
            anyhow::bail!("PIN setup failed: {}", result);
        }
    } else {
        println!("Skipping PIN setup. Device will remain unlocked.");
    }

    println!("\nSetting backup passphrase...");
    println!("The backup passphrase is used to encrypt OnlyKey backups.");
    println!("It must be at least 25 characters. Keep it somewhere safe.\n");

    let passphrase = loop {
        let p = rpassword::prompt_password("Backup passphrase (25+ chars): ")?;
        if p.len() < 25 {
            eprintln!(
                "Error: Passphrase must be at least 25 characters ({} given)",
                p.len()
            );
            continue;
        }
        let confirm = rpassword::prompt_password("Confirm backup passphrase: ")?;
        if p != confirm {
            eprintln!("Error: Passphrases do not match. Try again.");
            continue;
        }
        break p;
    };

    let key = protocol::hash_backup_passphrase(&passphrase);
    let mut priv_payload = vec![protocol::BACKUP_KEY_TYPE];
    priv_payload.extend(&key);
    dev.send_message(
        Message::SetPriv,
        Some(protocol::BACKUP_KEY_SLOT),
        None,
        &priv_payload,
    )?;
    std::thread::sleep(Duration::from_millis(500));
    let backup_result = dev.read_string(2000)?;
    if !backup_result.is_empty() {
        println!("Device: {}", backup_result);
    }

    if backup_result.contains("Error") {
        anyhow::bail!("Backup passphrase setup failed: {}", backup_result);
    }

    println!("\nDevice initialized successfully!");
    Ok(())
}

pub fn cmd_unlock(dev: &OnlyKeyDevice) -> Result<()> {
    if dev.device_type == DeviceType::Classic {
        anyhow::bail!(
            "Classic OnlyKey must be unlocked via physical button presses. \
             Enter your PIN on the device."
        );
    }

    let response = dev.handshake_raw()?;
    if response.contains("UNLOCKED") {
        println!("Device is already unlocked.");
        return Ok(());
    }
    if response.contains("UNINITIALIZED") {
        anyhow::bail!("Device is not initialized. Run 'okman init' first.");
    }

    let pin = rpassword::prompt_password("Enter PIN: ")?;
    let payload = protocol::encode_duo_unlock_payload(&pin);

    dev.send_message(Message::SetPin, None, None, &payload)?;
    std::thread::sleep(Duration::from_millis(1500));
    let result = dev.read_string(2000)?;
    if !result.is_empty() {
        println!("Device: {}", result);
    }

    if result.contains("Error") {
        anyhow::bail!("Unlock failed: {}", result);
    }

    Ok(())
}

pub fn cmd_wipe(dev: &OnlyKeyDevice, slot: &str, profile: Option<Profile>) -> Result<()> {
    let slot_id = protocol::parse_slot(slot, dev.device_type, profile)?;
    let slot_name = protocol::slot_name(slot_id, dev.device_type);

    let responses = wipe_slot(dev, slot_id)?;
    for r in &responses {
        println!("{}", r);
    }
    println!("Slot {} wiped.", slot_name);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_options_has_any_all_none() {
        let opts = ConfigOptions {
            lock_timeout: None,
            led_brightness: None,
            keyboard_layout: None,
            type_speed: None,
            lock_button: None,
            wipe_mode: None,
            touch_sense: None,
            sysadmin_mode: None,
            backup_mode: None,
            derived_challenge_mode: None,
            stored_challenge_mode: None,
            hmac_mode: None,
            second_profile_mode: None,
        };
        assert!(!opts.has_any());
    }

    #[test]
    fn config_options_has_any_one_set() {
        let opts = ConfigOptions {
            lock_timeout: Some(30),
            led_brightness: None,
            keyboard_layout: None,
            type_speed: None,
            lock_button: None,
            wipe_mode: None,
            touch_sense: None,
            sysadmin_mode: None,
            backup_mode: None,
            derived_challenge_mode: None,
            stored_challenge_mode: None,
            hmac_mode: None,
            second_profile_mode: None,
        };
        assert!(opts.has_any());
    }

    #[test]
    fn config_options_has_any_last_field() {
        let opts = ConfigOptions {
            lock_timeout: None,
            led_brightness: None,
            keyboard_layout: None,
            type_speed: None,
            lock_button: None,
            wipe_mode: None,
            touch_sense: None,
            sysadmin_mode: None,
            backup_mode: None,
            derived_challenge_mode: None,
            stored_challenge_mode: None,
            hmac_mode: None,
            second_profile_mode: Some(1),
        };
        assert!(opts.has_any());
    }

    #[test]
    fn config_slot_id_type_speed_is_zero() {
        assert_eq!(config_slot_id(MessageField::KeyTypeSpeed), 0);
    }

    #[test]
    fn config_slot_id_other_fields_are_one() {
        assert_eq!(config_slot_id(MessageField::IdleTimeout), 1);
        assert_eq!(config_slot_id(MessageField::LedBrightness), 1);
        assert_eq!(config_slot_id(MessageField::KeyLayout), 1);
        assert_eq!(config_slot_id(MessageField::LockButton), 1);
        assert_eq!(config_slot_id(MessageField::WipeMode), 1);
        assert_eq!(config_slot_id(MessageField::TouchSense), 1);
        assert_eq!(config_slot_id(MessageField::SysadminMode), 1);
        assert_eq!(config_slot_id(MessageField::BackupMode), 1);
        assert_eq!(config_slot_id(MessageField::DerivedChallengeMode), 1);
        assert_eq!(config_slot_id(MessageField::StoredChallengeMode), 1);
        assert_eq!(config_slot_id(MessageField::HmacMode), 1);
        assert_eq!(config_slot_id(MessageField::SecProfileMode), 1);
    }

    fn make_opts(label: Option<&str>, password: Option<&str>, generate: bool) -> SetOptions {
        SetOptions {
            label: label.map(|s| s.to_string()),
            username: None,
            password: password.map(|s| s.to_string()),
            generate,
            enter_after_password: false,
            no_enter_after_password: false,
        }
    }

    #[test]
    fn requires_label_password_without_label() {
        assert!(requires_label(&make_opts(None, Some("secret"), false)));
    }

    #[test]
    fn requires_label_generate_without_label() {
        assert!(requires_label(&make_opts(None, None, true)));
    }

    #[test]
    fn requires_label_password_with_label() {
        assert!(!requires_label(&make_opts(
            Some("GitHub"),
            Some("secret"),
            false
        )));
    }

    #[test]
    fn requires_label_generate_with_label() {
        assert!(!requires_label(&make_opts(Some("GitHub"), None, true)));
    }

    #[test]
    fn requires_label_no_password_no_label() {
        assert!(!requires_label(&make_opts(None, None, false)));
    }

    #[test]
    fn requires_label_label_only() {
        assert!(!requires_label(&make_opts(Some("GitHub"), None, false)));
    }
}
