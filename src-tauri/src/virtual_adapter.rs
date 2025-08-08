// src-tauri/src/virtual_adapter.rs
use anyhow::{Context, Result};
use pnet_datalink::{self, Channel, Config, DataLinkSender, DataLinkReceiver};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tun::{Device, DeviceBuilder};

use crate::interface_manager::InterfaceManager;
use crate::packet_router::{PacketRouter, LoadBalancingMode};
use crate::performance_monitor::PerformanceMonitor;

pub struct VirtualNetworkInterface {
    tun_interface: Box<dyn Device>,
    packet_router: Arc<PacketRouter>,
    performance_monitor: Arc<PerformanceMonitor>,
    is_running: Arc<tokio::sync::RwLock<bool>>,
}

impl VirtualNetworkInterface {
    pub async fn new() -> Result<Self> {
        println!("Creating virtual network interface...");
        
        // Create TUN interface
        let tun = DeviceBuilder::new()
            .name("NetBoost-TUN".to_string())
            .tap(false) // Use TUN mode (layer 3)
            .mtu(1500)
            .build()
            .await
            .context("Failed to create TUN interface")?;

        println!("Virtual network interface '{}' created.", tun.name());

        // Initialize interface manager
        let interface_manager = InterfaceManager::new()
            .context("Failed to initialize interface manager")?;

        // Create packet router
        let packet_router = Arc::new(PacketRouter::new(interface_manager));

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
        let reader_handle = self.spawn_packet_reader(packet_tx).await?;

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
        // Note: This is a simplified version. In a real implementation,
        // we'd need to handle the TUN interface reading differently
        // since tun-rs doesn't directly support tokio async reading
        
        let is_running = Arc::clone(&self.is_running);
        
        let handle = tokio::spawn(async move {
            let mut buf = [0u8; 1504]; // MTU + some overhead
            
            while *is_running.read().await {
                // Simulate packet reading - in real implementation,
                // this would read from the TUN interface
                tokio::time::sleep(Duration::from_millis(10)).await;
                
                // For now, just create a dummy packet to test the pipeline
                let dummy_packet = vec![0u8; 60]; // Minimum Ethernet frame size
                
                if packet_tx.send(dummy_packet).await.is_err() {
                    println!("Packet receiver dropped");
                    break;
                }
            }
        });

        Ok(handle)
    }

    async fn process_packet(
        packet_data: Vec<u8>,
        packet_router: &PacketRouter,
        performance_monitor: &PerformanceMonitor,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();

        // Record packet received
        performance_monitor.record_packet_received(packet_data.len()).await;

        // Route the packet
        match packet_router.route_packet(&packet_data).await {
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
        // This is where we would actually send the packet to the physical interface
        // For now, we'll just simulate it
        
        println!(
            "Sending {} bytes to interface {} (index: {})",
            packet_data.len(),
            routing_decision.interface_name,
            routing_decision.interface_index
        );

        // In a real implementation, this would:
        // 1. Create a raw socket or use pnet_datalink
        // 2. Send the packet through the selected physical interface
        // 3. Handle any errors or retries

        // Simulate network delay
        tokio::time::sleep(Duration::from_millis(1)).await;

        Ok(())
    }

    async fn start_performance_monitoring(&self) -> tokio::task::JoinHandle<()> {
        let performance_monitor = Arc::clone(&self.performance_monitor);
        let packet_router = Arc::clone(&self.packet_router);
        let is_running = Arc::clone(&self.is_running);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));
            
            while *is_running.read().await {
                interval.tick().await;
                
                // Update interface metrics
                let stats = performance_monitor.get_current_stats().await;
                
                // For now, simulate metrics updates
                // In real implementation, this would ping interfaces and measure actual performance
                packet_router.update_interface_metrics(
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
        // Create a new packet router with the updated mode
        // This is a simplified approach - in practice, you'd want to update the existing router
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
}

impl Drop for VirtualNetworkInterface {
    fn drop(&mut self) {
        println!("Virtual network interface dropped");
    }
}