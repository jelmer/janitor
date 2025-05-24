//! Performance monitoring for the runner.

use crate::metrics::MetricsCollector;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Performance monitoring manager for tracking system performance.
pub struct PerformanceMonitor {
    metrics: Arc<MetricsCollector>,
    performance_data: Arc<RwLock<PerformanceData>>,
    collection_interval: Duration,
}

/// Internal performance data storage.
#[derive(Debug, Default)]
struct PerformanceData {
    cpu_usage_history: Vec<f64>,
    memory_usage_history: Vec<u64>,
    disk_io_history: Vec<DiskIoStats>,
    network_io_history: Vec<NetworkIoStats>,
    database_performance: Vec<DatabasePerfStats>,
    queue_performance: Vec<QueuePerfStats>,
    last_collection: Option<Instant>,
}

/// CPU usage statistics.
#[derive(Debug, Clone, Copy)]
pub struct CpuStats {
    pub usage_percent: f64,
    pub load_average: [f64; 3], // 1min, 5min, 15min
    pub cores: usize,
}

/// Memory usage statistics.
#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub buffer_cache_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

/// Disk I/O statistics.
#[derive(Debug, Clone)]
pub struct DiskIoStats {
    pub timestamp: Instant,
    pub read_bytes_per_sec: u64,
    pub write_bytes_per_sec: u64,
    pub read_iops: u64,
    pub write_iops: u64,
    pub disk_usage_percent: f64,
}

/// Network I/O statistics.
#[derive(Debug, Clone)]
pub struct NetworkIoStats {
    pub timestamp: Instant,
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
    pub rx_packets_per_sec: u64,
    pub tx_packets_per_sec: u64,
    pub connections_active: u64,
}

/// Database performance statistics.
#[derive(Debug, Clone)]
pub struct DatabasePerfStats {
    pub timestamp: Instant,
    pub connection_pool_active: u32,
    pub connection_pool_idle: u32,
    pub avg_query_duration_ms: f64,
    pub queries_per_second: f64,
    pub slow_queries_count: u64,
}

/// Queue performance statistics.
#[derive(Debug, Clone)]
pub struct QueuePerfStats {
    pub timestamp: Instant,
    pub pending_items: u64,
    pub processing_items: u64,
    pub avg_processing_time_ms: f64,
    pub throughput_per_minute: f64,
    pub backlog_age_minutes: f64,
}

/// Performance thresholds for alerting.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceThresholds {
    pub cpu_usage_warning: f64,
    pub cpu_usage_critical: f64,
    pub memory_usage_warning: f64,
    pub memory_usage_critical: f64,
    pub disk_usage_warning: f64,
    pub disk_usage_critical: f64,
    pub queue_backlog_warning_minutes: f64,
    pub queue_backlog_critical_minutes: f64,
    pub database_slow_query_threshold_ms: f64,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            cpu_usage_warning: 80.0,
            cpu_usage_critical: 95.0,
            memory_usage_warning: 85.0,
            memory_usage_critical: 95.0,
            disk_usage_warning: 85.0,
            disk_usage_critical: 95.0,
            queue_backlog_warning_minutes: 30.0,
            queue_backlog_critical_minutes: 120.0,
            database_slow_query_threshold_ms: 1000.0,
        }
    }
}

/// Performance alert levels.
#[derive(Debug, Clone, PartialEq)]
pub enum AlertLevel {
    Ok,
    Warning,
    Critical,
}

/// Performance alert.
#[derive(Debug, Clone)]
pub struct PerformanceAlert {
    pub component: String,
    pub metric: String,
    pub level: AlertLevel,
    pub current_value: f64,
    pub threshold: f64,
    pub message: String,
    pub timestamp: Instant,
}

impl PerformanceMonitor {
    /// Create a new performance monitor.
    pub fn new(collection_interval: Duration) -> Self {
        Self {
            metrics: Arc::new(MetricsCollector {}),
            performance_data: Arc::new(RwLock::new(PerformanceData::default())),
            collection_interval,
        }
    }

    /// Start the performance monitoring loop.
    pub async fn start_monitoring(&self, thresholds: PerformanceThresholds) {
        let mut interval = tokio::time::interval(self.collection_interval);
        let performance_data = Arc::clone(&self.performance_data);
        let metrics = Arc::clone(&self.metrics);

        tokio::spawn(async move {
            loop {
                interval.tick().await;
                
                if let Err(e) = Self::collect_performance_metrics(&performance_data, &metrics, &thresholds).await {
                    log::error!("Failed to collect performance metrics: {}", e);
                }
            }
        });
    }

    /// Collect all performance metrics.
    async fn collect_performance_metrics(
        performance_data: &Arc<RwLock<PerformanceData>>,
        metrics: &Arc<MetricsCollector>,
        thresholds: &PerformanceThresholds,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let now = Instant::now();
        
        // Collect system metrics
        let cpu_stats = Self::collect_cpu_stats().await?;
        let memory_stats = Self::collect_memory_stats().await?;
        let disk_io_stats = Self::collect_disk_io_stats().await?;
        let network_io_stats = Self::collect_network_io_stats().await?;

        // Update Prometheus metrics
        crate::metrics::MEMORY_USAGE_BYTES
            .with_label_values(&["used"])
            .set(memory_stats.used_bytes as i64);
        
        crate::metrics::MEMORY_USAGE_BYTES
            .with_label_values(&["available"])
            .set(memory_stats.available_bytes as i64);

        // Store historical data
        {
            let mut data = performance_data.write().await;
            data.cpu_usage_history.push(cpu_stats.usage_percent);
            data.memory_usage_history.push(memory_stats.used_bytes);
            data.disk_io_history.push(disk_io_stats.clone());
            data.network_io_history.push(network_io_stats);
            data.last_collection = Some(now);

            // Keep only the last 1000 entries for each metric
            const MAX_HISTORY: usize = 1000;
            if data.cpu_usage_history.len() > MAX_HISTORY {
                let excess = data.cpu_usage_history.len() - MAX_HISTORY;
                data.cpu_usage_history.drain(0..excess);
            }
            if data.memory_usage_history.len() > MAX_HISTORY {
                let excess = data.memory_usage_history.len() - MAX_HISTORY;
                data.memory_usage_history.drain(0..excess);
            }
            if data.disk_io_history.len() > MAX_HISTORY {
                let excess = data.disk_io_history.len() - MAX_HISTORY;
                data.disk_io_history.drain(0..excess);
            }
            if data.network_io_history.len() > MAX_HISTORY {
                let excess = data.network_io_history.len() - MAX_HISTORY;
                data.network_io_history.drain(0..excess);
            }
        }

        // Check thresholds and generate alerts
        Self::check_performance_thresholds(&cpu_stats, &memory_stats, &disk_io_stats, thresholds);

        Ok(())
    }

    /// Collect CPU statistics.
    async fn collect_cpu_stats() -> Result<CpuStats, Box<dyn std::error::Error + Send + Sync>> {
        // Simple implementation - in production, this would use sysinfo or similar
        let load_avg = std::fs::read_to_string("/proc/loadavg")
            .unwrap_or_default()
            .split_whitespace()
            .take(3)
            .map(|s| s.parse::<f64>().unwrap_or(0.0))
            .collect::<Vec<_>>();

        let load_average = [
            load_avg.get(0).copied().unwrap_or(0.0),
            load_avg.get(1).copied().unwrap_or(0.0),
            load_avg.get(2).copied().unwrap_or(0.0),
        ];

        // Estimate CPU usage from load average
        let cores = num_cpus::get();
        let usage_percent = (load_average[0] / cores as f64 * 100.0).min(100.0);

        Ok(CpuStats {
            usage_percent,
            load_average,
            cores,
        })
    }

    /// Collect memory statistics.
    async fn collect_memory_stats() -> Result<MemoryStats, Box<dyn std::error::Error + Send + Sync>> {
        let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let mut values = HashMap::new();

        for line in meminfo.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let value = value.trim()
                    .split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .parse::<u64>()
                    .unwrap_or(0);
                values.insert(key, value * 1024); // Convert kB to bytes
            }
        }

        let total_bytes = values.get("MemTotal").copied().unwrap_or(0);
        let available_bytes = values.get("MemAvailable").copied().unwrap_or(0);
        let buffer_cache_bytes = values.get("Buffers").copied().unwrap_or(0) + 
                                values.get("Cached").copied().unwrap_or(0);
        let swap_total_bytes = values.get("SwapTotal").copied().unwrap_or(0);
        let swap_used_bytes = swap_total_bytes - values.get("SwapFree").copied().unwrap_or(0);

        Ok(MemoryStats {
            total_bytes,
            used_bytes: total_bytes - available_bytes,
            available_bytes,
            buffer_cache_bytes,
            swap_total_bytes,
            swap_used_bytes,
        })
    }

    /// Collect disk I/O statistics.
    async fn collect_disk_io_stats() -> Result<DiskIoStats, Box<dyn std::error::Error + Send + Sync>> {
        // Simplified implementation
        Ok(DiskIoStats {
            timestamp: Instant::now(),
            read_bytes_per_sec: 0,
            write_bytes_per_sec: 0,
            read_iops: 0,
            write_iops: 0,
            disk_usage_percent: 50.0, // Placeholder
        })
    }

    /// Collect network I/O statistics.
    async fn collect_network_io_stats() -> Result<NetworkIoStats, Box<dyn std::error::Error + Send + Sync>> {
        // Simplified implementation
        Ok(NetworkIoStats {
            timestamp: Instant::now(),
            rx_bytes_per_sec: 0,
            tx_bytes_per_sec: 0,
            rx_packets_per_sec: 0,
            tx_packets_per_sec: 0,
            connections_active: 0,
        })
    }

    /// Check performance thresholds and generate alerts.
    fn check_performance_thresholds(
        cpu_stats: &CpuStats,
        memory_stats: &MemoryStats,
        disk_io_stats: &DiskIoStats,
        thresholds: &PerformanceThresholds,
    ) {
        // CPU threshold checking
        if cpu_stats.usage_percent >= thresholds.cpu_usage_critical {
            log::error!("Critical CPU usage: {:.1}%", cpu_stats.usage_percent);
        } else if cpu_stats.usage_percent >= thresholds.cpu_usage_warning {
            log::warn!("High CPU usage: {:.1}%", cpu_stats.usage_percent);
        }

        // Memory threshold checking
        let memory_usage_percent = (memory_stats.used_bytes as f64 / memory_stats.total_bytes as f64) * 100.0;
        if memory_usage_percent >= thresholds.memory_usage_critical {
            log::error!("Critical memory usage: {:.1}%", memory_usage_percent);
        } else if memory_usage_percent >= thresholds.memory_usage_warning {
            log::warn!("High memory usage: {:.1}%", memory_usage_percent);
        }

        // Disk threshold checking
        if disk_io_stats.disk_usage_percent >= thresholds.disk_usage_critical {
            log::error!("Critical disk usage: {:.1}%", disk_io_stats.disk_usage_percent);
        } else if disk_io_stats.disk_usage_percent >= thresholds.disk_usage_warning {
            log::warn!("High disk usage: {:.1}%", disk_io_stats.disk_usage_percent);
        }
    }

    /// Get current performance summary.
    pub async fn get_performance_summary(&self) -> PerformanceSummary {
        let data = self.performance_data.read().await;
        
        PerformanceSummary {
            cpu_usage_current: data.cpu_usage_history.last().copied().unwrap_or(0.0),
            cpu_usage_avg_1h: data.cpu_usage_history.iter().rev().take(60).sum::<f64>() / 60.0,
            memory_usage_current: data.memory_usage_history.last().copied().unwrap_or(0),
            memory_usage_peak_1h: data.memory_usage_history.iter().rev().take(60).max().copied().unwrap_or(0),
            disk_io_avg_1h: data.disk_io_history.iter().rev().take(60).map(|s| s.read_bytes_per_sec + s.write_bytes_per_sec).sum::<u64>() / 60,
            network_io_avg_1h: data.network_io_history.iter().rev().take(60).map(|s| s.rx_bytes_per_sec + s.tx_bytes_per_sec).sum::<u64>() / 60,
            data_points_collected: data.cpu_usage_history.len(),
            last_collection: data.last_collection,
        }
    }

    /// Record database performance metrics.
    pub async fn record_database_performance(&self, stats: DatabasePerfStats) {
        let mut data = self.performance_data.write().await;
        data.database_performance.push(stats);

        // Keep only the last 1000 entries
        const MAX_HISTORY: usize = 1000;
        if data.database_performance.len() > MAX_HISTORY {
            let excess = data.database_performance.len() - MAX_HISTORY;
            data.database_performance.drain(0..excess);
        }
    }

    /// Record queue performance metrics.
    pub async fn record_queue_performance(&self, stats: QueuePerfStats) {
        let mut data = self.performance_data.write().await;
        data.queue_performance.push(stats);

        // Keep only the last 1000 entries
        const MAX_HISTORY: usize = 1000;
        if data.queue_performance.len() > MAX_HISTORY {
            let excess = data.queue_performance.len() - MAX_HISTORY;
            data.queue_performance.drain(0..excess);
        }
    }
}

/// Performance summary for monitoring dashboards.
#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    pub cpu_usage_current: f64,
    pub cpu_usage_avg_1h: f64,
    pub memory_usage_current: u64,
    pub memory_usage_peak_1h: u64,
    pub disk_io_avg_1h: u64,
    pub network_io_avg_1h: u64,
    pub data_points_collected: usize,
    pub last_collection: Option<Instant>,
}

/// Performance monitoring configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceConfig {
    pub collection_interval: Duration,
    pub thresholds: PerformanceThresholds,
    pub enable_detailed_logging: bool,
    pub alert_cooldown: Duration,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            collection_interval: Duration::from_secs(30),
            thresholds: PerformanceThresholds::default(),
            enable_detailed_logging: false,
            alert_cooldown: Duration::from_secs(5 * 60),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_thresholds_default() {
        let thresholds = PerformanceThresholds::default();
        assert_eq!(thresholds.cpu_usage_warning, 80.0);
        assert_eq!(thresholds.cpu_usage_critical, 95.0);
    }

    #[tokio::test]
    async fn test_performance_monitor_creation() {
        let monitor = PerformanceMonitor::new(Duration::from_secs(60));
        let summary = monitor.get_performance_summary().await;
        assert_eq!(summary.data_points_collected, 0);
    }

    #[test]
    fn test_alert_level_comparison() {
        assert_eq!(AlertLevel::Ok, AlertLevel::Ok);
        assert_ne!(AlertLevel::Warning, AlertLevel::Critical);
    }

    #[tokio::test]
    async fn test_database_performance_recording() {
        let monitor = PerformanceMonitor::new(Duration::from_secs(60));
        
        let stats = DatabasePerfStats {
            timestamp: Instant::now(),
            connection_pool_active: 5,
            connection_pool_idle: 10,
            avg_query_duration_ms: 250.0,
            queries_per_second: 100.0,
            slow_queries_count: 2,
        };

        monitor.record_database_performance(stats).await;
        
        // Verify that the stats were recorded
        let data = monitor.performance_data.read().await;
        assert_eq!(data.database_performance.len(), 1);
    }
}