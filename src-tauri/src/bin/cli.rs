// src/bin/cli.rs
use clap::Parser;
use netboost_pro_lib::InterfaceManager;

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

    /// List all available interfaces
    #[arg(short, long)]
    list: bool,
}

fn main() {
    env_logger::init();
    
    let args = Args::parse();

    if args.start {
        println!("Starting NetBoost Pro service...");
        println!("Note: Full service implementation requires GUI mode.");
        println!("Run the main application for full functionality.");
    } else if args.discover || args.list {
        println!("Discovering network interfaces...");
        match InterfaceManager::new() {
            Ok(manager) => {
                let interfaces = manager.get_all_interfaces();
                
                if interfaces.is_empty() {
                    println!("No network interfaces found.");
                } else {
                    println!("Found {} network interface(s):", interfaces.len());
                    println!();
                    
                    for (i, interface) in interfaces.iter().enumerate() {
                        println!("Interface {}:", i + 1);
                        println!("  Name: {}", interface.name);
                        println!("  Description: {}", interface.description);
                        println!("  IP Address: {}", interface.ip_address);
                        println!("  Index: {}", interface.index);
                        println!();
                    }
                    
                    if let Some(primary) = manager.get_primary_interface() {
                        println!("Primary interface: {} ({})", primary.name, primary.ip_address);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error discovering interfaces: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        println!("NetBoost Pro CLI");
        println!("Use --help for available commands.");
        println!("Available options:");
        println!("  --discover  Discover and list network interfaces");
        println!("  --list      List all available interfaces");
        println!("  --start     Start the NetBoost Pro service (limited in CLI mode)");
    }
}