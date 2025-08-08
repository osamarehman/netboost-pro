use anyhow::{Context, Result};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

use crate::interface_manager::{PhysicalInterface, InterfaceManager};

#[derive(Debug, Clone)]
pub struct PacketMetrics {
    pub latency: Duration,
    pub bandwidth_usage: u64,
    pub packet_loss: f32,
    pub last_updated: Instant,
}

#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub interface_index: u32,
    pub interface_name: String,
    pub confidence: f32, // 0.0 to 1.0
    pub reason: String,
}

#[derive(Debug, Clone, Copy)]
pub enum TrafficType {
    Gaming,      // Low latency priority
    Streaming,   // High bandwidth priority  
    File,        // Best effort
    Web,         // Balanced
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub enum LoadBalancingMode {
    RoundRobin,
    LatencyBased,
    BandwidthBased,
    Balanced,
}

pub struct PacketRouter {
    interface_manager: Arc<InterfaceManager>,
    interface_metrics: Arc<RwLock<HashMap<u32, PacketMetrics>>>,
    routing_table: Arc<RwLock<HashMap<Ipv4Addr, u32>>>,
    load_balancing_mode: LoadBalancingMode,
    round_robin_counter: Arc<RwLock<usize>>,
}

impl PacketRouter {
    pub fn new(interface_manager: InterfaceManager) -> Self {
        Self {
            interface_manager: Arc::new(interface_manager),
            interface_metrics: Arc::new(RwLock::new(HashMap::new())),
            routing_table: Arc::new(RwLock::new(HashMap::new())),
            load_balancing_mode: LoadBalancingMode::Balanced,
            round_robin_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Analyze incoming packet and determine optimal routing
    pub async fn route_packet(&self, packet_data: &[u8]) -> Result<RoutingDecision> {
        // Simplified packet analysis for development
        let traffic_info = self.analyze_packet_simple(packet_data)?;
        
        // Get current interface metrics
        let metrics = self.interface_metrics.read().await;
        let available_interfaces = self.get_available_interfaces().await;

        if available_interfaces.is_empty() {
            return Err(anyhow::anyhow!("No available interfaces for routing"));
        }

        // Apply load balancing strategy
        let selected_interface = match self.load_balancing_mode {
            LoadBalancingMode::RoundRobin => {
                self.select_round_robin(&available_interfaces).await
            }
            LoadBalancingMode::LatencyBased => {
                self.select_by_latency(&available_interfaces, &metrics).await
            }
            LoadBalancingMode::BandwidthBased => {
                self.select_by_bandwidth(&available_interfaces, &metrics).await
            }
            LoadBalancingMode::Balanced => {
                self.select_balanced(&available_interfaces, &metrics, traffic_info.traffic_type).await
            }
        };

        let interface = selected_interface.context("Failed to select interface")?;
        
        Ok(RoutingDecision {
            interface_index: interface.index,
            interface_name: interface.name.clone(),
            confidence: self.calculate_confidence(&interface, &metrics).await,
            reason: format!("Selected based on {:?} strategy", self.load_balancing_mode),
        })
    }

    /// Simplified packet analysis without deep packet inspection
    fn analyze_packet_simple(&self, packet_data: &[u8]) -> Result<TrafficInfo> {
        // For development, we'll do basic analysis based on packet size and patterns
        let packet_size = packet_data.len() as u64;
        
        let (traffic_type, priority) = match packet_size {
            0..=64 => (TrafficType::Gaming, 4),      // Small packets often gaming/VoIP
            65..=512 => (TrafficType::Web, 2),       // Medium packets often web traffic
            513..=1500 => (TrafficType::Streaming, 3), // Large packets often streaming
            _ => (TrafficType::File, 1),             // Very large packets often file transfer
        };

        Ok(TrafficInfo {
            traffic_type,
            priority,
            estimated_size: packet_size,
            destination: None, // Would need actual packet parsing for this
        })
    }

    /// Round-robin interface selection
    async fn select_round_robin(&self, interfaces: &[PhysicalInterface]) -> Option<PhysicalInterface> {
        let mut counter = self.round_robin_counter.write().await;
        let index = *counter % interfaces.len();
        *counter += 1;
        interfaces.get(index).cloned()
    }

    /// Select interface with lowest latency
    async fn select_by_latency(&self, interfaces: &[PhysicalInterface], metrics: &HashMap<u32, PacketMetrics>) -> Option<PhysicalInterface> {
        interfaces.iter()
            .min_by(|a, b| {
                let latency_a = metrics.get(&a.index)
                    .map(|m| m.latency)
                    .unwrap_or(Duration::from_millis(9999));
                let latency_b = metrics.get(&b.index)
                    .map(|m| m.latency)
                    .unwrap_or(Duration::from_millis(9999));
                latency_a.cmp(&latency_b)
            })
            .cloned()
    }

    /// Select interface with highest available bandwidth
    async fn select_by_bandwidth(&self, interfaces: &[PhysicalInterface], metrics: &HashMap<u32, PacketMetrics>) -> Option<PhysicalInterface> {
        interfaces.iter()
            .max_by(|a, b| {
                let usage_a = metrics.get(&a.index)
                    .map(|m| m.bandwidth_usage)
                    .unwrap_or(u64::MAX);
                let usage_b = metrics.get(&b.index)
                    .map(|m| m.bandwidth_usage)
                    .unwrap_or(u64::MAX);
                // Lower usage = higher available bandwidth
                usage_b.cmp(&usage_a)
            })
            .cloned()
    }

    /// Balanced selection based on traffic type
    async fn select_balanced(&self, interfaces: &[PhysicalInterface], metrics: &HashMap<u32, PacketMetrics>, traffic_type: TrafficType) -> Option<PhysicalInterface> {
        match traffic_type {
            TrafficType::Gaming => {
                // Prioritize latency for gaming
                self.select_by_latency(interfaces, metrics).await
            }
            TrafficType::Streaming => {
                // Prioritize bandwidth for streaming
                self.select_by_bandwidth(interfaces, metrics).await
            }
            TrafficType::File => {
                // Use least loaded interface for file transfers
                self.select_by_bandwidth(interfaces, metrics).await
            }
            TrafficType::Web | TrafficType::Unknown => {
                // Balanced approach for web traffic
                self.select_weighted_best(interfaces, metrics).await
            }
        }
    }

    /// Weighted selection considering both latency and bandwidth
    async fn select_weighted_best(&self, interfaces: &[PhysicalInterface], metrics: &HashMap<u32, PacketMetrics>) -> Option<PhysicalInterface> {
        interfaces.iter()
            .max_by(|a, b| {
                let score_a = self.calculate_interface_score(a, metrics);
                let score_b = self.calculate_interface_score(b, metrics);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Calculate a composite score for interface selection
    fn calculate_interface_score(&self, interface: &PhysicalInterface, metrics: &HashMap<u32, PacketMetrics>) -> f32 {
        if let Some(metric) = metrics.get(&interface.index) {
            let latency_score = 1000.0 / (metric.latency.as_millis() as f32 + 1.0);
            let bandwidth_score = 1.0 / (metric.bandwidth_usage as f32 + 1.0);
            let reliability_score = 1.0 - metric.packet_loss;
            
            // Weighted combination
            (latency_score * 0.4) + (bandwidth_score * 0.4) + (reliability_score * 0.2)
        } else {
            0.0 // No metrics available
        }
    }

    async fn get_available_interfaces(&self) -> Vec<PhysicalInterface> {
        // Return all interfaces from the interface manager
        self.interface_manager.get_all_interfaces().clone()
    }

    async fn calculate_confidence(&self, interface: &PhysicalInterface, metrics: &HashMap<u32, PacketMetrics>) -> f32 {
        if let Some(metric) = metrics.get(&interface.index) {
            // Base confidence on metrics quality
            let latency_confidence = if metric.latency.as_millis() < 50 { 0.9 } else { 0.6 };
            let loss_confidence = 1.0 - metric.packet_loss;
            (latency_confidence + loss_confidence) / 2.0
        } else {
            0.5 // Medium confidence when no metrics available
        }
    }

    /// Update metrics for an interface
    pub async fn update_interface_metrics(&self, interface_index: u32, latency: Duration, bandwidth_usage: u64, packet_loss: f32) {
        let mut metrics = self.interface_metrics.write().await;
        metrics.insert(interface_index, PacketMetrics {
            latency,
            bandwidth_usage,
            packet_loss,
            last_updated: Instant::now(),
        });
    }

    /// Set load balancing mode
    pub fn set_load_balancing_mode(&mut self, mode: LoadBalancingMode) {
        self.load_balancing_mode = mode;
    }
}

#[derive(Debug)]
struct TrafficInfo {
    traffic_type: TrafficType,
    priority: u8,
    estimated_size: u64,
    destination: Option<Ipv4Addr>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_round_robin_selection() {
        // Test implementation for round-robin selection
        // This would require setting up mock interfaces
    }

    #[tokio::test]
    async fn test_packet_classification() {
        // Test packet classification logic
        // This would require creating test packet data
    }
}