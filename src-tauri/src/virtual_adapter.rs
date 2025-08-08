use anyhow::Result;
use tun::{Device, DeviceBuilder};

pub struct VirtualNetworkInterface {
    tun_interface: Box<dyn Device>,
}

impl VirtualNetworkInterface {
    pub async fn new() -> Result<Self> {
        let tun = DeviceBuilder::new()
            .name("NetBoost-TUN".to_string())
            .tap(false) // Use TUN mode (layer 3)
            .mtu(1500)
            .build()
            .await?;

        println!("Virtual network interface '{}' created.", tun.name());

        Ok(Self { tun_interface: tun })
    }

    pub async fn run(mut self) {
        println!("Virtual network interface is running.");
        let mut buf = [0u8; 1504]; // MTU
        loop {
            match self.tun_interface.recv(&mut buf).await {
                Ok(n) => {
                    println!("Read {} bytes from TUN interface", n);
                    // For now, we just print the packet size. Later, this is where
                    // the packet will be routed to a physical interface.
                }
                Err(e) => {
                    eprintln!("Error reading from TUN interface: {}", e);
                    break;
                }
            }
        }
    }
}
