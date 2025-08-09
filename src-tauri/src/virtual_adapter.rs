// src-tauri/src/virtual_adapter.rs
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{Duration, interval};

use crate::interface_manager::InterfaceManager;
use crate::packet_router::{PacketRouter, LoadBalancingMode};
use crate::performance_monitor::PerformanceMonitor;
use pnet_datalink::{self, Channel};
use std::net::Ipv4Addr;

use tun::{DeviceBuilder, AsyncDevice};

struct TunInterface {
    device: Arc<AsyncDevice>,
}

impl TunInterface {
    async fn new(name: &str) -> Result<Self> {
        let address: Ipv4Addr = "10.0.0.1".parse()?;
        let dev = DeviceBuilder::new()
            .name(name.to_string())
            .ipv4(address, 24, None)
            .build_async()?;

        println!("Created TUN interface: {}", dev.name()?);

        Ok(Self {
            device: Arc::new(dev),
        })
    }

    fn name(&self) -> Result<String> {
        self.device.name().map_err(anyhow::Error::from)
    }
}

pub struct VirtualNetworkInterface {
    tun_interface: TunInterface,
    packet_router: Arc<RwLock<PacketRouter>>,
    performance_monitor: Arc<PerformanceMonitor>,
    is_running: Arc<tokio::sync::RwLock<bool>>,
}

impl VirtualNetworkInterface {
    pub async fn new() -> Result<Self> {
        println!("Creating virtual network interface...");
        
        // Create TUN interface
        let tun = TunInterface::new("NetBoost-TUN")
            .await
            .context("Failed to create TUN interface")?;

        println!("Virtual network interface '{}' created.", tun.name()?);

        // Initialize interface manager
        let interface_manager = InterfaceManager::new()
            .context("Failed to initialize interface manager")?;

        // Create packet router
        let packet_router = Arc::new(RwLock::new(PacketRouter::new(interface_manager)));

        // Create performance monitor
        let performance_monitor = Arc::new(PerformanceMonitor::new());

        Ok(Self {
            tun_interface: tun,
            packet_router,
            performance_monitor,
            is_running: Arc::new(tokio::sync::RwLock::new(false)),
        })
    }

    pub async fn run(mut self) -> Result<()> {
        println!("Starting NetBoost Pro virtual network interface...");
        
        // Set running state
        *self.is_running.write().await = true;

        // Start performance monitoring
        let monitor_handle = self.start_performance_monitoring().await;

        // Start packet processing
        let packet_handle = self.start_packet_processing().await?;

        // Wait for shutdown signal or error
        tokio::select! {
            result = packet_handle => {
                println!("Packet processing ended: {:?}", result);
            }
            _ = monitor_handle => {
                println!("Performance monitoring ended");
            }
        }

        // Clean shutdown
        *self.is_running.write().await = false;
        println!("NetBoost Pro virtual interface stopped.");
        
        Ok(())
    }

    async fn start_packet_processing(&mut self) -> Result<tokio::task::JoinHandle<Result<()>>> {
        let packet_router = Arc::clone(&self.packet_router);
        let performance_monitor = Arc::clone(&self.performance_monitor);
        let is_running = Arc::clone(&self.is_running);

        // Create channels for packet processing
        let (packet_tx, mut packet_rx) = mpsc::channel::<Vec<u8>>(1000);
        
        // Spawn packet reader task
        let _reader_handle = self.spawn_packet_reader(packet_tx).await?;

        // Main packet processing task
        let handle = tokio::spawn(async move {
            println!("Packet processing loop started");
            
            while *is_running.read().await {
                tokio::select! {
                    Some(packet_data) = packet_rx.recv() => {
                        if let Err(e) = Self::process_packet(
                            packet_data,
                            &packet_router,
                            &performance_monitor
                        ).await {
                            eprintln!("Error processing packet: {}", e);
                        }
                    }
                    else => {
                        println!("Packet channel closed");
                        break;
                    }
                }
            }

            println!("Packet processing loop ended");
            Ok(())
        });

        Ok(handle)
    }

    async fn spawn_packet_reader(&mut self, packet_tx: mpsc::Sender<Vec<u8>>) -> Result<tokio::task::JoinHandle<()>> {
        let is_running = Arc::clone(&self.is_running);
        let device: Arc<AsyncDevice> = Arc::clone(&self.tun_interface.device);
        
        let handle = tokio::spawn(async move {
            let mut buf = [0u8; 1504]; // MTU + some overhead
            
            while *is_running.read().await {
                match device.recv(&mut buf).await {
                    Ok(len) => {
                        if packet_tx.send(buf[..len].to_vec()).await.is_err() {
                            println!("Packet receiver dropped");
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading from TUN device: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(handle)
    }

    async fn process_packet(
        packet_data: Vec<u8>,
        packet_router: &Arc<RwLock<PacketRouter>>,
        performance_monitor: &PerformanceMonitor,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();

        // Record packet received
        performance_monitor.record_packet_received(packet_data.len()).await;

        // Route the packet
        match packet_router.read().await.route_packet(&packet_data).await {
            Ok(routing_decision) => {
                println!(
                    "Routing packet to interface '{}' (confidence: {:.2}%): {}",
                    routing_decision.interface_name,
                    routing_decision.confidence * 100.0,
                    routing_decision.reason
                );

                // Send packet to selected interface
                if let Err(e) = Self::send_packet_to_interface(&packet_data, &routing_decision).await {
                    eprintln!("Failed to send packet to interface: {}", e);
                    performance_monitor.record_packet_dropped().await;
                } else {
                    performance_monitor.record_packet_forwarded(packet_data.len()).await;
                }
            }
            Err(e) => {
                eprintln!("Failed to route packet: {}", e);
                performance_monitor.record_packet_dropped().await;
            }
        }

        // Record processing time
        let processing_time = start_time.elapsed();
        performance_monitor.record_processing_latency(processing_time).await;

        Ok(())
    }

    async fn send_packet_to_interface(
        packet_data: &[u8],
        routing_decision: &crate::packet_router::RoutingDecision,
    ) -> Result<()> {
        let interfaces = pnet_datalink::interfaces();
        let interface = interfaces
            .into_iter()
            .find(|iface| iface.index == routing_decision.interface_index)
            .context("Failed to find the selected interface")?;

        let (mut tx, _) = match pnet_datalink::channel(&interface, Default::default()) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => return Err(anyhow::anyhow!("Unsupported channel type")),
            Err(e) => return Err(e.into()),
        };

        tx.send_to(packet_data, None)
            .context("Failed to send packet")?
            .context("Failed to send packet")?;

        Ok(())
    }

    async fn start_performance_monitoring(&self) -> tokio::task::JoinHandle<()> {
        let performance_monitor = Arc::clone(&self.performance_monitor);
        let packet_router: Arc<RwLock<PacketRouter>> = Arc::clone(&self.packet_router);
        let is_running = Arc::clone(&self.is_running);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));
            
            while *is_running.read().await {
                interval.tick().await;
                
                // Update interface metrics
                let stats = performance_monitor.get_current_stats().await;
                
                // For now, simulate metrics updates
                // In real implementation, this would ping interfaces and measure actual performance
                packet_router.write().await.update_interface_metrics(
                    1, // interface index
                    Duration::from_millis(20), // simulated latency
                    stats.bandwidth_usage,
                    stats.packet_loss_rate,
                ).await;

                // Log performance stats
                println!(
                    "Performance Stats - Packets: {}/{}/{}, Latency: {:.2}ms, Loss: {:.2}%",
                    stats.packets_received,
                    stats.packets_forwarded,
                    stats.packets_dropped,
                    stats.average_latency.as_secs_f64() * 1000.0,
                    stats.packet_loss_rate * 100.0
                );
            }
        })
    }

    /// Configure load balancing mode
    pub async fn set_load_balancing_mode(&mut self, mode: LoadBalancingMode) {
        self.packet_router.write().await.set_load_balancing_mode(mode);
        println!("Load balancing mode changed to: {:?}", mode);
    }

    /// Get current performance statistics
    pub async fn get_performance_stats(&self) -> crate::performance_monitor::PerformanceStats {
        self.performance_monitor.get_current_stats().await
    }

    /// Stop the virtual interface
    pub async fn stop(&self) {
        println!("Stopping virtual network interface...");
        *self.is_running.write().await = false;
    }

    pub fn name(&self) -> Result<String> {
        self.tun_interface.name()
    }
}

impl Drop for VirtualNetworkInterface {
    fn drop(&mut self) {
        println!("Virtual network interface dropped");
    }
}