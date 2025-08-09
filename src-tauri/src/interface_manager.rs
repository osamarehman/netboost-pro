use anyhow::Result;
use std::net::Ipv4Addr;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhysicalInterface {
    pub name: String,
    pub description: String,
    pub ip_address: Ipv4Addr,
    pub index: u32,
}

pub struct InterfaceManager {
    pub interfaces: Vec<PhysicalInterface>,
}

impl InterfaceManager {
    pub fn new() -> Result<Self> {
        let mut manager = Self {
            interfaces: Vec::new(),
        };
        manager.discover_interfaces()?;
        Ok(manager)
    }

    fn discover_interfaces(&mut self) -> Result<()> {
        println!("Discovering network interfaces...");
        
        self.interfaces = pnet_datalink::interfaces()
            .into_iter()
            .filter(|iface| iface.is_up() && !iface.is_loopback() && !iface.ips.is_empty())
            .filter_map(|iface| {
                iface.ips.iter().find(|ip| ip.is_ipv4()).map(|ip| {
                    let ip_addr = match ip.ip() {
                        std::net::IpAddr::V4(ipv4) => ipv4,
                        _ => return None, // Should not happen due to filter
                    };
                    Some(PhysicalInterface {
                        name: iface.name.clone(),
                        description: iface.description.clone(),
                        ip_address: ip_addr,
                        index: iface.index,
                    })
                }).flatten()
            })
            .collect();

        println!("Found {} interfaces:", self.interfaces.len());
        for iface in &self.interfaces {
            println!("  - {}: {} (index {})", iface.name, iface.ip_address, iface.index);
        }

        Ok(())
    }

    pub fn get_primary_interface(&self) -> Option<&PhysicalInterface> {
        self.interfaces.first()
    }

    pub fn get_all_interfaces(&self) -> &Vec<PhysicalInterface> {
        &self.interfaces
    }
}

// Future implementation ideas for real interface discovery:
#[cfg(windows)]
mod windows_impl {
    // Use Windows API directly:
    // - GetAdaptersAddresses
    // - WMI queries
    // - ipconfig parsing
}

#[cfg(unix)]
mod unix_impl {
    // Use Unix-specific methods:
    // - Parse /proc/net/dev (Linux)
    // - Use getifaddrs (macOS/BSD)
    // - Parse ip route show (Linux)
}