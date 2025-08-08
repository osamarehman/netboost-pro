use clap::Parser;
use netboost_pro_lib::interface_manager::InterfaceManager;

/// NetBoost Pro Command-Line Interface
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Start the NetBoost Pro service
    #[arg(short, long)]
    start: bool,

    /// Discover and list network interfaces
    #[arg(short, long)]
    discover: bool,
}

fn main() {
    let args = Args::parse();

    if args.start {
        println!("Starting NetBoost Pro service...");
        // Later, this will initialize and run the VirtualNetworkInterface
    } else if args.discover {
        println!("Discovering network interfaces...");
        match InterfaceManager::new() {
            Ok(manager) => {
                if let Some(primary) = manager.get_primary_interface() {
                    println!("Primary interface found: {:?}", primary);
                } else {
                    println!("No suitable primary interface found.");
                }
            }
            Err(e) => {
                eprintln!("Error discovering interfaces: {}", e);
            }
        }
    } else {
        println!("No command specified. Use --help for options.");
    }
}
