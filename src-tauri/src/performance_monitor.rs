use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceStats {
    pub packets_received: u64,
    pub packets_forwarded: u64,
    pub packets_dropped: u64,
    pub bandwidth_usage: u64,
    pub average_latency: Duration,
    pub packet_loss_rate: f32,
    pub uptime: Duration,
}

pub struct PerformanceMonitor {
    stats: Arc<RwLock<InternalStats>>,
    start_time: Instant,
}

#[derive(Debug)]
struct InternalStats {
    packets_received: u64,
    packets_forwarded: u64,
    packets_dropped: u64,
    total_bytes_received: u64,
    total_bytes_forwarded: u64,
    total_processing_time: Duration,
    latency_samples: Vec<Duration>,
    max_latency_samples: usize,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(InternalStats {
                packets_received: 0,
                packets_forwarded: 0,
                packets_dropped: 0,
                total_bytes_received: 0,
                total_bytes_forwarded: 0,
                total_processing_time: Duration::new(0, 0),
                latency_samples: Vec::new(),
                max_latency_samples: 1000, // Keep last 1000 samples
            })),
            start_time: Instant::now(),
        }
    }

    pub async fn record_packet_received(&self, bytes: usize) {
        let mut stats = self.stats.write().await;
        stats.packets_received += 1;
        stats.total_bytes_received += bytes as u64;
    }

    pub async fn record_packet_forwarded(&self, bytes: usize) {
        let mut stats = self.stats.write().await;
        stats.packets_forwarded += 1;
        stats.total_bytes_forwarded += bytes as u64;
    }

    pub async fn record_packet_dropped(&self) {
        let mut stats = self.stats.write().await;
        stats.packets_dropped += 1;
    }

    pub async fn record_processing_latency(&self, latency: Duration) {
        let mut stats = self.stats.write().await;
        stats.total_processing_time += latency;
        
        // Add latency sample and maintain a rolling window
        stats.latency_samples.push(latency);
        if stats.latency_samples.len() > stats.max_latency_samples {
            stats.latency_samples.remove(0);
        }
    }

    pub async fn get_current_stats(&self) -> PerformanceStats {
        let stats = self.stats.read().await;
        let uptime = self.start_time.elapsed();

        // Calculate average latency from samples
        let average_latency = if !stats.latency_samples.is_empty() {
            let total_latency: Duration = stats.latency_samples.iter().sum();
            total_latency / stats.latency_samples.len() as u32
        } else {
            Duration::new(0, 0)
        };

        // Calculate packet loss rate
        let packet_loss_rate = if stats.packets_received > 0 {
            stats.packets_dropped as f32 / stats.packets_received as f32
        } else {
            0.0
        };

        // Calculate bandwidth usage (bytes per second)
        let bandwidth_usage = if uptime.as_secs() > 0 {
            stats.total_bytes_forwarded / uptime.as_secs()
        } else {
            0
        };

        PerformanceStats {
            packets_received: stats.packets_received,
            packets_forwarded: stats.packets_forwarded,
            packets_dropped: stats.packets_dropped,
            bandwidth_usage,
            average_latency,
            packet_loss_rate,
            uptime,
        }
    }

    #[allow(dead_code)]
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = InternalStats {
            packets_received: 0,
            packets_forwarded: 0,
            packets_dropped: 0,
            total_bytes_received: 0,
            total_bytes_forwarded: 0,
            total_processing_time: Duration::new(0, 0),
            latency_samples: Vec::new(),
            max_latency_samples: stats.max_latency_samples,
        };
    }
}