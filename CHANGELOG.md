# Changelog

## 0.2.2

### Changed

- `okman set` now requires `--label` when setting a password on a slot that doesn't have one — the device only reports labels, so unlabeled slots are invisible to `okman list`

## 0.2.1

### Fixed

- DUO no-PIN mode — device returns `UNINITIALIZED...n` when configured without PIN; now correctly treated as unlocked
- Slot table column width — dynamic sizing for DUO slot names (e.g. "green 1a")
- Re-running `okman init` on an already-initialized no-PIN DUO now gives a clear error

### Changed

- `slot_name()` returns `Cow<'static, str>` to avoid allocation for Classic slot names
- `validate_pin()` returns `Result<(), OnlyKeyError>` instead of `Result<(), String>`
- Removed unused `parse_profile()` — profile parsing handled by clap `ValueEnum`
- Replaced magic number `48` with `b'0'` in PIN encoding

## 0.2.0

### Added

- **OnlyKey DUO support** — auto-detected at connection time via USB serial number
- `okman init` for DUO — PINs entered via software (optional, skip for always-unlocked mode), backup passphrase (required, 25+ chars)
- `okman unlock` — unlock a DUO with PIN set, via software PIN entry
- `--profile` flag on `list`, `set`, `wipe` — select DUO color profile (green, blue, yellow, purple)
- DUO slot mapping: 3 buttons × short/long press × 4 profiles = 24 slots
- `--password [VALUE]` — now accepts an inline value for scripting; bare `--password` still prompts interactively

### Fixed

- `--enter-after-password` — was sending wrong protocol value (USB HID scan code 128 instead of OnlyKey protocol value 2)

### Changed

- `parse_slot()` and `slot_name()` are now device-aware (Classic vs DUO)
- Error messages are device-aware (e.g. DUO locked → "Run `okman unlock`", Classic locked → "Enter PIN on device")
- `InvalidSlot` errors now include device-specific valid slot formats

### Dependencies

- Added `sha2` for backup passphrase hashing during DUO init

## 0.1.3

- Device config commands (`okman config`)
- Additional slot field support (NextKey, delays, touch sensitivity, etc.)

## 0.1.2

- Random password generation (`--generate`)

## 0.1.1

- Flag to clear enter-after-password (`--no-enter-after-password`)
