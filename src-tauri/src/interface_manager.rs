use anyhow::{Context, Result};
use pnet_datalink::NetworkInterface;
use std::net::Ipv4Addr;

#[derive(Debug, Clone)]
pub struct PhysicalInterface {
    pub name: String,
    pub description: String,
    pub ip_address: Ipv4Addr,
    pub index: u32,
}

pub struct InterfaceManager {
    interfaces: Vec<PhysicalInterface>,
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
        let all_interfaces = pnet_datalink::interfaces();

        self.interfaces = all_interfaces
            .into_iter()
            .filter_map(|iface: NetworkInterface| {
                if iface.is_up() && !iface.is_loopback() {
                    iface.ips.iter().find(|ip| ip.is_ipv4()).map(|ip| {
                        let ip_addr = match ip.ip() {
                            std::net::IpAddr::V4(ipv4) => ipv4,
                            _ => unreachable!(),
                        };
                        Some(PhysicalInterface {
                            name: iface.name.clone(),
                            description: iface.description.clone(),
                            ip_address: ip_addr,
                            index: iface.index,
                        })
                    }).flatten()
                } else {
                    None
                }
            })
            .collect();

        println!("Found {} suitable interfaces:", self.interfaces.len());
        for iface in &self.interfaces {
            println!("  - {}: {} (index {})", iface.name, iface.ip_address, iface.index);
        }

        Ok(())
    }

    pub fn get_primary_interface(&self) -> Option<&PhysicalInterface> {
        self.interfaces.first()
    }
}
