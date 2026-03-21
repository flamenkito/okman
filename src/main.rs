mod commands;
mod device;
mod error;
mod protocol;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use device::OnlyKeyDevice;
use protocol::MessageField;

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
        #[arg(long, short)]
        password: bool,
        #[arg(long)]
        enter_after_password: bool,
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
            let labels = commands::get_labels(&dev)?;
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
        }

        Commands::Set {
            slot,
            label,
            username,
            password,
            enter_after_password,
        } => {
            let slot_id = protocol::parse_slot(&slot)?;
            let slot_name = protocol::slot_name(slot_id);
            let dev = connect_and_handshake()?;

            if let Some(ref l) = label {
                let resp = commands::set_slot_field(&dev, slot_id, MessageField::Label, l)?;
                println!("Label set for slot {}. Device: {}", slot_name, resp);
            }

            if let Some(ref u) = username {
                let resp = commands::set_slot_field(&dev, slot_id, MessageField::Username, u)?;
                println!("Username set for slot {}. Device: {}", slot_name, resp);
            }

            if password {
                let pw =
                    rpassword::prompt_password(format!("Enter password for slot {}: ", slot_name))?;

                if pw.is_empty() {
                    anyhow::bail!("Password cannot be empty");
                }

                let resp = commands::set_slot_field(&dev, slot_id, MessageField::Password, &pw)?;
                println!("Password set for slot {}. Device: {}", slot_name, resp);
            }

            if enter_after_password {
                let resp = commands::set_slot_field_raw(
                    &dev,
                    slot_id,
                    MessageField::NextKey2,
                    &[protocol::KEY_RETURN],
                )?;
                println!(
                    "Enter-after-password enabled for slot {}. Device: {}",
                    slot_name, resp
                );
            }

            if label.is_none() && username.is_none() && !password && !enter_after_password {
                anyhow::bail!(
                    "Nothing to set. Use --label, --username, --password, or --enter-after-password"
                );
            }
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
            let responses = commands::wipe_slot(&dev, slot_id)?;
            for r in &responses {
                println!("{}", r);
            }
            println!("Slot {} wiped.", slot_name);
        }
    }

    Ok(())
}
