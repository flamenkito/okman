mod commands;
mod device;
mod error;
mod password;
mod protocol;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};

use commands::{ConfigOptions, SetOptions};
use device::OnlyKeyDevice;

#[derive(Parser)]
#[command(
    name = "okman",
    version,
    about = "Manage OnlyKey via USB HID — slots, PINs, and device settings"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, ValueEnum)]
enum ProfileArg {
    Green,
    Blue,
    Yellow,
    Purple,
}

impl ProfileArg {
    fn to_protocol(&self) -> protocol::Profile {
        match self {
            ProfileArg::Green => protocol::Profile::Green,
            ProfileArg::Blue => protocol::Profile::Blue,
            ProfileArg::Yellow => protocol::Profile::Yellow,
            ProfileArg::Purple => protocol::Profile::Purple,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// List configured slots in a table
    List {
        /// DUO profile (green, blue, yellow, purple). Ignored for Classic.
        #[arg(long, short, value_enum)]
        profile: Option<ProfileArg>,
    },

    /// Set fields on a slot
    Set {
        /// Slot name (Classic: 1a-6a, 1b-6b. DUO: 1a-3a, 1b-3b)
        slot: String,
        #[arg(long, short)]
        label: Option<String>,
        #[arg(long, short)]
        username: Option<String>,
        /// Set password (prompts if no value given)
        #[arg(long, short, conflicts_with = "generate", num_args = 0..=1, default_missing_value = "")]
        password: Option<String>,
        /// Generate a random password (xxxxxx-xxxxxx-xxxxxx)
        #[arg(long, short, conflicts_with = "password")]
        generate: bool,
        #[arg(long, conflicts_with = "no_enter_after_password")]
        enter_after_password: bool,
        /// Disable pressing Enter after password
        #[arg(long, conflicts_with = "enter_after_password")]
        no_enter_after_password: bool,
        /// DUO profile (green, blue, yellow, purple). Ignored for Classic.
        #[arg(long, value_enum)]
        profile: Option<ProfileArg>,
    },

    /// Wipe all fields from a slot
    Wipe {
        /// Slot name (Classic: 1a-6a, 1b-6b. DUO: 1a-3a, 1b-3b)
        slot: String,
        /// DUO profile (green, blue, yellow, purple). Ignored for Classic.
        #[arg(long, value_enum)]
        profile: Option<ProfileArg>,
    },

    /// Initialize an uninitialized OnlyKey (set PINs)
    Init,

    /// Unlock an OnlyKey DUO by entering PIN via software
    Unlock,

    /// Configure device-level settings
    Config {
        /// Auto-lock after N minutes idle (0 = never, range 0-255)
        #[arg(long)]
        lock_timeout: Option<u8>,
        /// LED brightness (0-255)
        #[arg(long)]
        led_brightness: Option<u8>,
        /// USB keyboard layout code
        #[arg(long)]
        keyboard_layout: Option<u8>,
        /// Key type speed (0 = fastest, 10 = slowest)
        #[arg(long)]
        type_speed: Option<u8>,
        /// Lock button setting (0 = disable)
        #[arg(long)]
        lock_button: Option<u8>,
        /// Wipe mode (0 = off, 1 = on)
        #[arg(long)]
        wipe_mode: Option<u8>,
        /// Touch sensitivity
        #[arg(long)]
        touch_sense: Option<u8>,
        /// Sysadmin mode (0 = off, 1 = on)
        #[arg(long)]
        sysadmin_mode: Option<u8>,
        /// Backup key mode
        #[arg(long)]
        backup_mode: Option<u8>,
        /// Derived key challenge-response mode (0 = off, 1 = on)
        #[arg(long)]
        derived_challenge_mode: Option<u8>,
        /// Stored key challenge-response mode (0 = off, 1 = on)
        #[arg(long)]
        stored_challenge_mode: Option<u8>,
        /// HMAC challenge-response mode (0 = off, 1 = on)
        #[arg(long)]
        hmac_mode: Option<u8>,
        /// Second profile mode (0 = off, 1 = on)
        #[arg(long)]
        second_profile_mode: Option<u8>,
    },
}

fn connect_and_handshake() -> Result<OnlyKeyDevice> {
    eprintln!("Connecting to OnlyKey...");
    let dev = OnlyKeyDevice::open().context("Failed to connect to OnlyKey")?;
    match dev.handshake() {
        Ok(version) => {
            eprintln!("Connected ({}): {}", dev.device_type, version.trim());
            Ok(dev)
        }
        Err(crate::error::OnlyKeyError::DeviceUninitialized) => {
            anyhow::bail!(
                "{} has no PIN set. Run 'okman init' to set up your device.",
                dev.device_type
            );
        }
        Err(crate::error::OnlyKeyError::DeviceLocked) => match dev.device_type {
            device::DeviceType::Duo => {
                anyhow::bail!(
                    "{} is locked. Run 'okman unlock' to unlock with your PIN.",
                    dev.device_type
                );
            }
            device::DeviceType::Classic => {
                anyhow::bail!(
                    "{} is locked. Enter your PIN on the device first.",
                    dev.device_type
                );
            }
        },
        Err(e) => Err(e).context("Handshake failed"),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List { profile } => {
            let dev = connect_and_handshake()?;
            commands::cmd_list(&dev, profile.map(|p| p.to_protocol()))
        }

        Commands::Set {
            slot,
            label,
            username,
            password,
            generate,
            enter_after_password,
            no_enter_after_password,
            profile,
        } => {
            let dev = connect_and_handshake()?;
            commands::cmd_set(
                &dev,
                &slot,
                SetOptions {
                    label,
                    username,
                    password,
                    generate,
                    enter_after_password,
                    no_enter_after_password,
                },
                profile.map(|p| p.to_protocol()),
            )
        }

        Commands::Wipe { slot, profile } => {
            let dev = connect_and_handshake()?;
            let prof = profile.map(|p| p.to_protocol());
            let slot_id = protocol::parse_slot(&slot, dev.device_type, prof)?;
            let slot_name = protocol::slot_name(slot_id, dev.device_type);

            eprint!("Are you sure you want to wipe slot {}? [y/N]: ", slot_name);
            let mut confirm = String::new();
            std::io::stdin().read_line(&mut confirm)?;
            if confirm.trim().to_lowercase() != "y" {
                println!("Cancelled.");
                return Ok(());
            }

            commands::cmd_wipe(&dev, &slot, prof)
        }

        Commands::Init => {
            eprintln!("Connecting to OnlyKey...");
            let dev = OnlyKeyDevice::open().context("Failed to connect to OnlyKey")?;
            commands::cmd_init(&dev)
        }

        Commands::Unlock => {
            eprintln!("Connecting to OnlyKey...");
            let dev = OnlyKeyDevice::open().context("Failed to connect to OnlyKey")?;
            commands::cmd_unlock(&dev)
        }

        Commands::Config {
            lock_timeout,
            led_brightness,
            keyboard_layout,
            type_speed,
            lock_button,
            wipe_mode,
            touch_sense,
            sysadmin_mode,
            backup_mode,
            derived_challenge_mode,
            stored_challenge_mode,
            hmac_mode,
            second_profile_mode,
        } => {
            let dev = connect_and_handshake()?;
            commands::cmd_config(
                &dev,
                &ConfigOptions {
                    lock_timeout,
                    led_brightness,
                    keyboard_layout,
                    type_speed,
                    lock_button,
                    wipe_mode,
                    touch_sense,
                    sysadmin_mode,
                    backup_mode,
                    derived_challenge_mode,
                    stored_challenge_mode,
                    hmac_mode,
                    second_profile_mode,
                },
            )
        }
    }
}
