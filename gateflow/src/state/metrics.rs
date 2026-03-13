use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub struct AppMetrics {
    inner: Arc<MetricsInner>,
}

#[derive(Debug, Default)]
struct MetricsInner {
    requests_total: AtomicU64,
    requests_error_total: AtomicU64,
    rate_limited_total: AtomicU64,
    upstream_requests_total: AtomicU64,
    upstream_error_total: AtomicU64,
    upstream_latency_ms_total: AtomicU64,
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub requests_total: u64,
    pub requests_error_total: u64,
    pub rate_limited_total: u64,
    pub upstream_requests_total: u64,
    pub upstream_error_total: u64,
    pub upstream_latency_ms_total: u64,
}

impl Default for AppMetrics {
    fn default() -> Self {
        Self {
            inner: Arc::new(MetricsInner::default()),
        }
    }
}

impl AppMetrics {
    pub fn inc_requests_total(&self) {
        self.inner.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_requests_error_total(&self) {
        self.inner.requests_error_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_rate_limited_total(&self) {
        self.inner.rate_limited_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_upstream_requests_total(&self) {
        self.inner.upstream_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_upstream_error_total(&self) {
        self.inner.upstream_error_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_upstream_latency_ms(&self, latency_ms: u64) {
        self.inner
            .upstream_latency_ms_total
            .fetch_add(latency_ms, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            requests_total: self.inner.requests_total.load(Ordering::Relaxed),
            requests_error_total: self.inner.requests_error_total.load(Ordering::Relaxed),
            rate_limited_total: self.inner.rate_limited_total.load(Ordering::Relaxed),
            upstream_requests_total: self.inner.upstream_requests_total.load(Ordering::Relaxed),
            upstream_error_total: self.inner.upstream_error_total.load(Ordering::Relaxed),
            upstream_latency_ms_total: self.inner.upstream_latency_ms_total.load(Ordering::Relaxed),
        }
    }
}
