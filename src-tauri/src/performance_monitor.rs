// src-tauri/src/performance_monitor.rs
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub packets_received: u64,
    pub packets_forwarded: u64,
    pub packets_dropped: u64,
    pub bytes_received: u64,
    pub bytes_forwarded: u64,
    pub average_latency: Duration,
    pub packet_loss_rate: f32,
    pub bandwidth_usage: u64, // bytes per second
    pub uptime: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceStats {
    pub interface_name: String,
    pub interface_index: u32,
    pub packets_sent: u64,
    pub bytes_sent: u64,
    pub current_latency: Duration,
    pub packet_loss_rate: f32,
    pub is_active: bool,
    pub last_used: Option<Instant>,
}

#[derive(Debug)]
struct LatencyMeasurement {
    timestamp: Instant,
    latency: Duration,
}

#[derive(Debug)]
struct ThroughputMeasurement {
    timestamp: Instant,
    bytes: u64,
}

pub struct PerformanceMonitor {
    // Global statistics
    stats: Arc<RwLock<PerformanceStats>>,
    
    // Per-interface statistics
    interface_stats: Arc<RwLock<std::collections::HashMap<u32, InterfaceStats>>>,
    
    // Circular buffers for time-series data
    latency_history: Arc<RwLock<VecDeque<LatencyMeasurement>>>,
    throughput_history: Arc<RwLock<VecDeque<ThroughputMeasurement>>>,
    
    // Monitoring configuration
    max_history_size: usize,
    start_time: Instant,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(PerformanceStats {
                packets_received: 0,
                packets_forwarded: 0,
                packets_dropped: 0,
                bytes_received: 0,
                bytes_forwarded: 0,
                average_latency: Duration::from_millis(0),
                packet_loss_rate: 0.0,
                bandwidth_usage: 0,
                uptime: Duration::from_secs(0),
            })),
            interface_stats: Arc::new(RwLock::new(std::collections::HashMap::new())),
            latency_history: Arc::new(RwLock::new(VecDeque::new())),
            throughput_history: Arc::new(RwLock::new(VecDeque::new())),
            max_history_size: 1000, // Keep last 1000 measurements
            start_time: Instant::now(),
        }
    }

    /// Record a packet received from the TUN interface
    pub async fn record_packet_received(&self, bytes: usize) {
        let mut stats = self.stats.write().await;
        stats.packets_received += 1;
        stats.bytes_received += bytes as u64;
        stats.uptime = self.start_time.elapsed();
    }

    /// Record a packet successfully forwarded to a physical interface
    pub async fn record_packet_forwarded(&self, bytes: usize) {
        let mut stats = self.stats.write().await;
        stats.packets_forwarded += 1;
        stats.bytes_forwarded += bytes as u64;
        
        // Update bandwidth usage
        self.update_bandwidth_usage(bytes as u64).await;
    }

    /// Record a packet that was dropped (failed to route/send)
    pub async fn record_packet_dropped(&self) {
        let mut stats = self.stats.write().await;
        stats.packets_dropped += 1;
        
        // Update packet loss rate
        let total_packets = stats.packets_received;
        if total_packets > 0 {
            stats.packet_loss_rate = stats.packets_dropped as f32 / total_packets as f32;
        }
    }

    /// Record processing latency for a packet
    pub async fn record_processing_latency(&self, latency: Duration) {
        // Add to history
        let mut history = self.latency_history.write().await;
        history.push_back(LatencyMeasurement {
            timestamp: Instant::now(),
            latency,
        });

        // Maintain maximum history size
        if history.len() > self.max_history_size {
            history.pop_front();
        }

        // Update average latency
        self.update_average_latency().await;
    }

    /// Record data sent through a specific interface
    pub async fn record_interface_usage(&self, interface_index: u32, interface_name: String, bytes: u64) {
        let mut interface_stats = self.interface_stats.write().await;
        
        let stats = interface_stats.entry(interface_index).or_insert(InterfaceStats {
            interface_name: interface_name.clone(),
            interface_index,
            packets_sent: 0,
            bytes_sent: 0,
            current_latency: Duration::from_millis(0),
            packet_loss_rate: 0.0,
            is_active: true,
            last_used: None,
        });

        stats.packets_sent += 1;
        stats.bytes_sent += bytes;
        stats.last_used = Some(Instant::now());
        stats.interface_name = interface_name; // Update name in case it changed
    }

    /// Update latency for a specific interface
    pub async fn record_interface_latency(&self, interface_index: u32, latency: Duration) {
        let mut interface_stats = self.interface_stats.write().await;
        
        if let Some(stats) = interface_stats.get_mut(&interface_index) {
            stats.current_latency = latency;
        }
    }

    /// Mark an interface as active or inactive
    pub async fn set_interface_active(&self, interface_index: u32, is_active: bool) {
        let mut interface_stats = self.interface_stats.write().await;
        
        if let Some(stats) = interface_stats.get_mut(&interface_index) {
            stats.is_active = is_active;
        }
    }

    /// Get current overall performance statistics
    pub async fn get_current_stats(&self) -> PerformanceStats {
        let mut stats = self.stats.read().await.clone();
        stats.uptime = self.start_time.elapsed();
        stats
    }

    /// Get performance statistics for a specific interface
    pub async fn get_interface_stats(&self, interface_index: u32) -> Option<InterfaceStats> {
        let interface_stats = self.interface_stats.read().await;
        interface_stats.get(&interface_index).cloned()
    }

    /// Get all interface statistics
    pub async fn get_all_interface_stats(&self) -> Vec<InterfaceStats> {
        let interface_stats = self.interface_stats.read().await;
        interface_stats.values().cloned().collect()
    }

    /// Get latency history for graphing/analysis
    pub async fn get_latency_history(&self, duration: Duration) -> Vec<(Instant, Duration)> {
        let history = self.latency_history.read().await;
        let cutoff_time = Instant::now() - duration;
        
        history
            .iter()
            .filter(|measurement| measurement.timestamp >= cutoff_time)
            .map(|measurement| (measurement.timestamp, measurement.latency))
            .collect()
    }

    /// Get throughput history for graphing/analysis
    pub async fn get_throughput_history(&self, duration: Duration) -> Vec<(Instant, u64)> {
        let history = self.throughput_history.read().await;
        let cutoff_time = Instant::now() - duration;
        
        history
            .iter()
            .filter(|measurement| measurement.timestamp >= cutoff_time)
            .map(|measurement| (measurement.timestamp, measurement.bytes))
            .collect()
    }

    /// Reset all statistics (useful for testing or manual reset)
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = PerformanceStats {
            packets_received: 0,
            packets_forwarded: 0,
            packets_dropped: 0,
            bytes_received: 0,
            bytes_forwarded: 0,
            average_latency: Duration::from_millis(0),
            packet_loss_rate: 0.0,
            bandwidth_usage: 0,
            uptime: Duration::from_secs(0),
        };

        let mut interface_stats = self.interface_stats.write().await;
        interface_stats.clear();

        let mut latency_history = self.latency_history.write().await;
        latency_history.clear();

        let mut throughput_history = self.throughput_history.write().await;
        throughput_history.clear();

        // Reset start time
        // Note: In a real implementation, you might not want to reset start_time
        // self.start_time = Instant::now();
    }

    /// Generate a performance report
    pub async fn generate_report(&self) -> PerformanceReport {
        let stats = self.get_current_stats().await;
        let interface_stats = self.get_all_interface_stats().await;
        let recent_latency = self.get_latency_history(Duration::from_minutes(5)).await;
        let recent_throughput = self.get_throughput_history(Duration::from_minutes(5)).await;

        PerformanceReport {
            overall_stats: stats,
            interface_stats,
            avg_latency_5min: Self::calculate_average_latency(&recent_latency),
            avg_throughput_5min: Self::calculate_average_throughput(&recent_throughput),
            report_timestamp: Instant::now(),
        }
    }

    // Private helper methods

    async fn update_average_latency(&self) {
        let history = self.latency_history.read().await;
        
        if !history.is_empty() {
            let total_latency: Duration = history.iter().map(|m| m.latency).sum();
            let average = total_latency / history.len() as u32;
            
            drop(history); // Release read lock
            
            let mut stats = self.stats.write().await;
            stats.average_latency = average;
        }
    }

    async fn update_bandwidth_usage(&self, bytes: u64) {
        let mut throughput_history = self.throughput_history.write().await;
        throughput_history.push_back(ThroughputMeasurement {
            timestamp: Instant::now(),
            bytes,
        });

        // Maintain maximum history size
        if throughput_history.len() > self.max_history_size {
            throughput_history.pop_front();
        }

        // Calculate bandwidth usage over the last second
        let one_second_ago = Instant::now() - Duration::from_secs(1);
        let recent_bytes: u64 = throughput_history
            .iter()
            .filter(|m| m.timestamp >= one_second_ago)
            .map(|m| m.bytes)
            .sum();

        drop(throughput_history); // Release write lock

        let mut stats = self.stats.write().await;
        stats.bandwidth_usage = recent_bytes;
    }

    fn calculate_average_latency(measurements: &[(Instant, Duration)]) -> Duration {
        if measurements.is_empty() {
            return Duration::from_millis(0);
        }

        let total: Duration = measurements.iter().map(|(_, latency)| *latency).sum();
        total / measurements.len() as u32
    }

    fn calculate_average_throughput(measurements: &[(Instant, u64)]) -> u64 {
        if measurements.is_empty() {
            return 0;
        }

        let total: u64 = measurements.iter().map(|(_, bytes)| *bytes).sum();
        total / measurements.len() as u64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub overall_stats: PerformanceStats,
    pub interface_stats: Vec<InterfaceStats>,
    pub avg_latency_5min: Duration,
    pub avg_throughput_5min: u64,
    pub report_timestamp: Instant,
}

// Implement default for easier testing
impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_packet_statistics() {
        let monitor = PerformanceMonitor::new();
        
        // Record some packets
        monitor.record_packet_received(100).await;
        monitor.record_packet_forwarded(100).await;
        
        let stats = monitor.get_current_stats().await;
        assert_eq!(stats.packets_received, 1);
        assert_eq!(stats.packets_forwarded, 1);
        assert_eq!(stats.bytes_received, 100);
        assert_eq!(stats.bytes_forwarded, 100);
    }

    #[tokio::test]
    async fn test_latency_tracking() {
        let monitor = PerformanceMonitor::new();
        
        // Record some latency measurements
        monitor.record_processing_latency(Duration::from_millis(10)).await;
        monitor.record_processing_latency(Duration::from_millis(20)).await;
        monitor.record_processing_latency(Duration::from_millis(30)).await;
        
        let stats = monitor.get_current_stats().await;
        assert_eq!(stats.average_latency, Duration::from_millis(20));
    }

    #[tokio::test]
    async fn test_interface_statistics() {
        let monitor = PerformanceMonitor::new();
        
        monitor.record_interface_usage(1, "eth0".to_string(), 100).await;
        monitor.record_interface_latency(1, Duration::from_millis(15)).await;
        
        let interface_stats = monitor.get_interface_stats(1).await.unwrap();
        assert_eq!(interface_stats.interface_name, "eth0");
        assert_eq!(interface_stats.packets_sent, 1);
        assert_eq!(interface_stats.bytes_sent, 100);
        assert_eq!(interface_stats.current_latency, Duration::from_millis(15));
    }
}