# okman

Rust CLI to manage OnlyKey and OnlyKey DUO via USB HID — slots, PINs, and device settings.

[![CI](https://github.com/flamenkito/okman/actions/workflows/rust.yml/badge.svg)](https://github.com/flamenkito/okman/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/okman)](https://crates.io/crates/okman)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

```
user@MacBook ~ % okman list
Connecting to OnlyKey...
Connected (OnlyKey DUO): UNLOCKEDv3.0.4-prodp
┌────────────┬───────────┐
│ Slot       │ Label     │
├────────────┼───────────┤
│ green 1a   │ GitHub    │
│ green 2a   │ AWS       │
│ green 3b   │ VPN       │
└────────────┴───────────┘
```

## Supported devices

| Device | Buttons | Slots | Profiles |
| ------ | ------- | ----- | -------- |
| OnlyKey Classic | 6 | 12 (1a–6a, 1b–6b) | 2 (primary + plausible deniability) |
| OnlyKey DUO | 3 | 24 (1a–3a, 1b–3b × 4 profiles) | 4 (green, blue, yellow, purple) |

Both are auto-detected at connection time.

## Why okman?

The official [OnlyKey App](https://github.com/trustcrypto/OnlyKey-App) is an Electron desktop app — great for initial setup, but heavyweight for day-to-day slot management. The [python-onlykey](https://github.com/trustcrypto/python-onlykey) CLI works but requires a Python runtime and pip dependencies.

okman is a single static binary with no runtime dependencies. It manages OnlyKey slots, PINs, and device settings from the terminal.

- **No runtime** — no Python, no Node.js, no Electron
- **Single binary** — `cargo install okman` or drop the binary in your PATH
- **Fast** — connects and executes in milliseconds
- **Scriptable** — easy to integrate into dotfiles, provisioning scripts, or CI

| Tool | Size | Runtime required |
| ---- | ---- | ---------------- |
| **okman** | **~1 MB** | None |
| python-onlykey | ~50 KB + ~100 MB Python | Python 3 + pip |
| OnlyKey App | ~200 MB | Electron (bundled) |

## Install

```bash
cargo install okman
```

Or build from source:

```bash
git clone https://github.com/flamenkito/okman.git
cd okman
cargo build --release
# binary at target/release/okman
```

## Prerequisites

- OnlyKey or OnlyKey DUO connected over USB
- For most commands the device must be unlocked:
  - **Classic** — enter the PIN on the physical device buttons
  - **DUO with PIN** — run `okman unlock`
  - **DUO without PIN** — always unlocked, no action needed
- `okman init` requires an uninitialized device (no PIN set yet)

### Linux permissions

You will likely need udev rules so your user can access the HID device without root:

```text
# /etc/udev/rules.d/49-onlykey.rules
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="16c0", ATTRS{idProduct}=="0486", MODE="0660", GROUP="plugdev"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1d50", ATTRS{idProduct}=="60fc", MODE="0660", GROUP="plugdev"
```

## Usage

### Initialize a new device

```bash
okman init
```

**Classic:** PINs are entered on the physical device buttons — they are never sent over USB. The wizard walks you through setting the primary PIN, second profile PIN, and self-destruct PIN. Each PIN must be 7–10 digits entered on the OnlyKey buttons.

**DUO:** PINs are entered in the terminal and sent to the device over USB. The wizard prompts for:

1. **Device PIN** (optional) — skip to leave the device always unlocked, ideal for keyboard-emulated password use
2. **Self-destruct PIN** (optional) — factory-resets the device when entered
3. **Backup passphrase** (required, 25+ characters) — encrypts device backups. The device transitions from uninitialized to initialized only after this step.

### Unlock (DUO only)

```bash
okman unlock
```

Unlocks a DUO that has a PIN set by entering the PIN via software. Not needed if the DUO was set up without a PIN. Classic devices must be unlocked by pressing buttons on the physical device.

### List configured slots

```bash
okman list
okman list --profile blue    # DUO: show slots from a specific profile
```

### Set slot fields

All flags are optional, but at least one is required:

```bash
okman set 1a --label "GitHub" --username "alice@example.com"
okman set 1a --password                    # prompts interactively
okman set 1a --password "hunter2"          # sets directly (scriptable)
okman set 1a -l "GitHub" -u "alice" -p --enter-after-password
okman set 1a --generate
okman set 1a --no-enter-after-password
okman set 2b -l "Bank"
```

DUO profiles:

```bash
okman set 1a --label "Work" --profile blue
okman set 3b --password --profile purple
```

### Wipe a slot

Asks for confirmation:

```bash
okman wipe 1a
okman wipe 2b --profile yellow    # DUO
```

### Configure device settings

All flags are optional, but at least one is required. Multiple flags can be combined in a single command.

```bash
okman config --lock-timeout 30 --led-brightness 128
okman config --keyboard-layout 1
okman config --type-speed 4
okman config --wipe-mode 1
```

<details>
<summary>All available options</summary>

#### Lock timeout

```bash
okman config --lock-timeout 30
```

Auto-lock the device after N minutes of inactivity. When locked, the PIN must be re-entered on the physical device to unlock. Set to `0` to disable auto-lock. Range: 0–255 minutes.

#### LED brightness

```bash
okman config --led-brightness 128
```

Adjust the brightness of the OnlyKey status LEDs. `0` turns LEDs off entirely, `255` is maximum brightness. Useful in dark environments or to make the device less conspicuous. Range: 0–255.

#### Keyboard layout

```bash
okman config --keyboard-layout 1
```

Set the USB keyboard layout used when OnlyKey types out passwords and usernames. This must match the keyboard layout configured on the host operating system, otherwise special characters will be typed incorrectly. The default is US English (`1`). Common values:

| Value | Layout |
| ----- | ------ |
| 1 | US English |
| 2 | Canadian French |
| 3 | Canadian Multilingual |
| 4 | Danish |
| 5 | Finnish |
| 6 | French |
| 7 | French Belgian |
| 8 | French Swiss |
| 9 | German |
| 10 | German Mac |
| 11 | German Swiss |
| 12 | Icelandic |
| 13 | Irish |
| 14 | Italian |
| 15 | Norwegian |
| 16 | Portuguese |
| 17 | Portuguese Brazilian |
| 18 | Spanish |
| 19 | Spanish Latin America |
| 20 | Swedish |
| 21 | Turkish |
| 22 | United Kingdom |
| 23 | US International |
| 24 | Czech |
| 25 | Serbian Latin Only |
| 26 | Hungarian |
| 27 | Danish Mac |
| 28 | Dvorak |

#### Type speed

```bash
okman config --type-speed 4
```

Control how fast OnlyKey types out characters when filling in credentials. Higher values are slower. Increase this if characters are being dropped or garbled on slower systems. `0` is fastest, `10` is slowest. Default varies by firmware.

#### Lock button

```bash
okman config --lock-button 0
```

Configure which button (held for 5+ seconds) locks the device. By default, holding button 3 for 5+ seconds locks the OnlyKey. Set to `0` to disable the lock button.

#### Wipe mode

```bash
okman config --wipe-mode 1
```

When enabled (`1`), entering 10 incorrect PINs will factory-reset the device, erasing all stored data. When disabled (`0`), incorrect PINs will lock the device but not wipe it. This is a security trade-off: wipe mode protects against brute-force attacks but risks accidental data loss.

#### Touch sensitivity

```bash
okman config --touch-sense 1
```

Adjust the sensitivity of the capacitive touch buttons. Higher values require firmer presses. Useful if the device is registering accidental touches or not responding reliably.

#### Sysadmin mode

```bash
okman config --sysadmin-mode 1
```

When enabled (`1`), OnlyKey can type any character including modifier keys (Ctrl, Alt, etc.) by using special escape sequences in slot fields. This allows automating complex logins, system administration commands, and custom key sequences beyond simple username/password entry. When disabled (`0`), only standard characters are typed. Disabled by default for safety.

#### Backup mode

```bash
okman config --backup-mode 1
```

Control how the encrypted backup key is derived. `1` = key slot mode (backup key is stored in a key slot), `2` = passphrase mode (backup key is derived from a passphrase set during setup). This affects how `okman` and the OnlyKey App perform encrypted backups and restores.

#### Derived challenge mode

```bash
okman config --derived-challenge-mode 1
```

Control whether derived key challenge-response is enabled. When enabled (`1`), applications can request challenge-response operations using keys derived from the device's master key. When disabled (`0`), derived challenge requests are rejected. Used by SSH/GPG agents for key derivation.

#### Stored challenge mode

```bash
okman config --stored-challenge-mode 1
```

Control whether stored key challenge-response is enabled. When enabled (`1`), applications can request cryptographic operations using keys explicitly loaded into ECC/RSA key slots. When disabled (`0`), stored key challenge requests are rejected. Used by SSH/GPG agents and OpenPGP applications.

#### HMAC mode

```bash
okman config --hmac-mode 1
```

Control whether HMAC challenge-response is enabled. When enabled (`1`), applications can perform HMAC-SHA1 challenge-response using HMAC keys loaded onto the device. This is used by tools like KeePassXC for database unlocking. When disabled (`0`), HMAC challenge requests are rejected.

#### Second profile mode

```bash
okman config --second-profile-mode 1
```

Control access to the second profile. OnlyKey supports two profiles, each with its own set of 12 slots, unlocked by different PINs. When set to `1`, the second profile is enabled. When set to `0`, only the primary profile is accessible. This is part of the plausible deniability feature — the second profile can be used to store a separate set of credentials that is only accessible with the second PIN.

</details>

<details>
<summary>Slot mapping</summary>

**Classic (6 buttons)**

| Button | Short press | Long press |
| ------ | ----------- | ---------- |
| 1 | 1a | 1b |
| 2 | 2a | 2b |
| 3 | 3a | 3b |
| 4 | 4a | 4b |
| 5 | 5a | 5b |
| 6 | 6a | 6b |

Short press = slots 1–6, long press = slots 7–12.

**DUO (3 buttons, 4 profiles)**

| Button | Short press | Long press |
| ------ | ----------- | ---------- |
| 1 (near green light) | 1a | 1b |
| 2 (other side) | 2a | 2b |
| 3 (both at once) | 3a | 3b |

6 slots per profile × 4 profiles (green, blue, yellow, purple) = 24 slots total. Hold button 3 for 5+ seconds to switch profiles (LED changes color).

</details>

## Security

- **Classic:** PIN entry happens on the physical device and is never sent over USB.
- **DUO:** PINs are entered via software and sent over USB during init and unlock. This is the same as the official OnlyKey App. DUO can also be set up without a PIN for always-unlocked operation.
- The CLI refuses to operate if the device is locked (except `okman init` which requires an uninitialized device).
- Stored passwords cannot be read back over USB — they can only be typed out by pressing the physical button.

## License

[MIT](LICENSE)
