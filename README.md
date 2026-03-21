# okman

Rust CLI to manage OnlyKey slot passwords via USB HID.

Implements the same USB HID protocol as the official Python CLI (python-onlykey).

## Prerequisites

- Rust toolchain (stable)
- OnlyKey device connected over USB
- Device must be unlocked (enter the PIN on the physical device)

Linux permissions

- You will likely need udev rules so your user can access the HID device without root.
- Vendor/Product IDs supported by this CLI: `16c0:0486` and `1d50:60fc`.

Example udev rules:

```text
# /etc/udev/rules.d/49-onlykey.rules
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="16c0", ATTRS{idProduct}=="0486", MODE="0660", GROUP="plugdev"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1d50", ATTRS{idProduct}=="60fc", MODE="0660", GROUP="plugdev"
```

## Build

```bash
cargo build --release
```

Binary output: `target/release/okman`

## Slot mapping

Valid slot names: `1a-6a`, `1b-6b`.

| Button | Short press | Long press |
| ------ | ---------- | --------- |
| 1 | 1a (slot 1) | 1b (slot 7) |
| 2 | 2a (slot 2) | 2b (slot 8) |
| 3 | 3a (slot 3) | 3b (slot 9) |
| 4 | 4a (slot 4) | 4b (slot 10) |
| 5 | 5a (slot 5) | 5b (slot 11) |
| 6 | 6a (slot 6) | 6b (slot 12) |

## Usage

List configured slots:

```bash
okman list
```

Set slot fields (all flags are optional, but at least one is required):

```bash
# Set label and username
okman set 1a --label "GitHub" --username "alice@example.com"

# Set password (prompts securely)
okman set 1a --password

# Set everything at once, with Enter sent after password
okman set 1a -l "GitHub" -u "alice" -p --enter-after-password

# Set just a label
okman set 2b -l "Bank"
```

Wipe a slot (asks for confirmation):

```bash
okman wipe 1a
```

## Notes

- PIN entry happens on the physical device and is never sent over USB.
- The CLI refuses to operate if the device is locked or uninitialized.
- Stored passwords cannot be read back over USB — they can only be typed out by pressing the physical button.
- `--enter-after-password` configures the slot to press Enter after typing the password.
