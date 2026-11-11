//! Progress reporting for long-running operations. Values are measured from real
//! byte counts; speed and ETA are only reported once enough data exists.

use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub operation: String,
    pub stage: String,
    pub detail: Option<String>,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub bytes_per_second: Option<f64>,
    pub eta_seconds: Option<f64>,
    pub step: u32,
    pub steps: u32,
    pub done: bool,
}

pub type Sink = Arc<dyn Fn(Progress) + Send + Sync>;

#[derive(Clone)]
pub struct CancelToken(Arc<AtomicBool>);
impl CancelToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst)
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
    pub fn check(&self) -> crate::error::AppResult<()> {
        if self.is_cancelled() {
            Err(crate::error::AppError::Cancelled)
        } else {
            Ok(())
        }
    }
}
impl Default for CancelToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracks throughput over a sliding window and throttles emitted events.
pub struct Meter {
    samples: VecDeque<(Instant, u64)>,
    last_emit: Option<Instant>,
}
impl Meter {
    pub fn new() -> Self {
        Self { samples: VecDeque::new(), last_emit: None }
    }
    /// Record the session byte count (bytes actually transferred, not cached ones).
    pub fn sample(&mut self, transferred: u64) {
        let now = Instant::now();
        self.samples.push_back((now, transferred));
        while let Some((t, _)) = self.samples.front() {
            if now.duration_since(*t) > Duration::from_secs(4) && self.samples.len() > 2 {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }
    pub fn speed(&self) -> Option<f64> {
        let (t0, b0) = self.samples.front()?;
        let (t1, b1) = self.samples.back()?;
        let dt = t1.duration_since(*t0).as_secs_f64();
        if dt < 0.75 || b1 <= b0 {
            return None;
        }
        Some((b1 - b0) as f64 / dt)
    }
    pub fn should_emit(&mut self) -> bool {
        let now = Instant::now();
        match self.last_emit {
            Some(t) if now.duration_since(t) < Duration::from_millis(120) => false,
            _ => {
                self.last_emit = Some(now);
                true
            }
        }
    }
}
impl Default for Meter {
    fn default() -> Self {
        Self::new()
    }
}

pub fn eta(remaining: u64, speed: Option<f64>) -> Option<f64> {
    speed.filter(|s| *s > 1024.0).map(|s| remaining as f64 / s)
}
