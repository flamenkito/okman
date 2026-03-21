mod commands;
mod device;
mod error;
mod password;
mod protocol;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use commands::SetOptions;
use device::OnlyKeyDevice;

#[derive(Parser)]
#[command(name = "okman", version, about = "Manage OnlyKey slot passwords")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List configured slots in a table
    List,

    /// Set fields on a slot
    Set {
        /// Slot name (1a-6a for short press, 1b-6b for long press)
        slot: String,
        #[arg(long, short)]
        label: Option<String>,
        #[arg(long, short)]
        username: Option<String>,
        #[arg(long, short, conflicts_with = "generate")]
        password: bool,
        /// Generate a random password (xxxxxx-xxxxxx-xxxxxx)
        #[arg(long, short, conflicts_with = "password")]
        generate: bool,
        #[arg(long, conflicts_with = "no_enter_after_password")]
        enter_after_password: bool,
        /// Disable pressing Enter after password
        #[arg(long, conflicts_with = "enter_after_password")]
        no_enter_after_password: bool,
    },

    /// Wipe all fields from a slot
    Wipe {
        /// Slot name (1a-6a for short press, 1b-6b for long press)
        slot: String,
    },
}

fn connect_and_handshake() -> Result<OnlyKeyDevice> {
    eprintln!("Connecting to OnlyKey...");
    let dev = OnlyKeyDevice::open().context("Failed to connect to OnlyKey")?;
    let version = dev.handshake().context("Handshake failed")?;
    eprintln!("Connected: {}", version.trim());
    Ok(dev)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => {
            let dev = connect_and_handshake()?;
            commands::cmd_list(&dev)
        }

        Commands::Set {
            slot,
            label,
            username,
            password,
            generate,
            enter_after_password,
            no_enter_after_password,
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
            )
        }

        Commands::Wipe { slot } => {
            let slot_id = protocol::parse_slot(&slot)?;
            let slot_name = protocol::slot_name(slot_id);

            eprint!("Are you sure you want to wipe slot {}? [y/N]: ", slot_name);
            let mut confirm = String::new();
            std::io::stdin().read_line(&mut confirm)?;
            if confirm.trim().to_lowercase() != "y" {
                println!("Cancelled.");
                return Ok(());
            }

            let dev = connect_and_handshake()?;
            commands::cmd_wipe(&dev, &slot)
        }
    }
}
