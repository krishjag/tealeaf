//! Rate limiter implementation using token bucket and sliding window

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore, SemaphorePermit};

/// Rate limiter using a combination of token bucket and sliding window
pub struct RateLimiter {
    requests_per_minute: u32,
    tokens_per_minute: u32,
    /// Reserved for future use with semaphore-based limiting
    #[allow(dead_code)]
    request_semaphore: Arc<Semaphore>,
    last_requests: Arc<Mutex<VecDeque<Instant>>>,
    token_usage: Arc<Mutex<VecDeque<(Instant, u32)>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(requests_per_minute: u32, tokens_per_minute: u32) -> Self {
        Self {
            requests_per_minute,
            tokens_per_minute,
            request_semaphore: Arc::new(Semaphore::new(requests_per_minute as usize)),
            last_requests: Arc::new(Mutex::new(VecDeque::new())),
            token_usage: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Acquire permission to make a request
    pub async fn acquire(&self) -> RateLimitGuard {
        loop {
            // First check if we can make a request based on sliding window
            let wait_time = self.check_request_limit().await;
            if let Some(wait) = wait_time {
                tokio::time::sleep(wait).await;
                continue;
            }

            // Record the request
            let mut last = self.last_requests.lock().await;
            last.push_back(Instant::now());

            return RateLimitGuard {
                _permit: None,
            };
        }
    }

    /// Check if we can make a request, returns wait time if we need to wait
    async fn check_request_limit(&self) -> Option<Duration> {
        let mut last = self.last_requests.lock().await;
        let now = Instant::now();
        let window = Duration::from_secs(60);

        // Remove requests older than 1 minute
        while let Some(&front) = last.front() {
            if now.duration_since(front) > window {
                last.pop_front();
            } else {
                break;
            }
        }

        // Check if we're at the limit
        if last.len() >= self.requests_per_minute as usize {
            // Calculate how long to wait
            if let Some(&oldest) = last.front() {
                let elapsed = now.duration_since(oldest);
                if elapsed < window {
                    return Some(window - elapsed + Duration::from_millis(10));
                }
            }
        }

        None
    }

    /// Record token usage for rate limiting
    pub async fn record_tokens(&self, tokens: u32) {
        let mut usage = self.token_usage.lock().await;
        let now = Instant::now();
        let window = Duration::from_secs(60);

        // Remove old entries
        while let Some(&(time, _)) = usage.front() {
            if now.duration_since(time) > window {
                usage.pop_front();
            } else {
                break;
            }
        }

        usage.push_back((now, tokens));
    }

    /// Check current token usage in the last minute
    pub async fn current_token_usage(&self) -> u32 {
        let mut usage = self.token_usage.lock().await;
        let now = Instant::now();
        let window = Duration::from_secs(60);

        // Remove old entries
        while let Some(&(time, _)) = usage.front() {
            if now.duration_since(time) > window {
                usage.pop_front();
            } else {
                break;
            }
        }

        usage.iter().map(|(_, t)| t).sum()
    }

    /// Check if we have token capacity
    pub async fn has_token_capacity(&self, needed: u32) -> bool {
        let current = self.current_token_usage().await;
        current + needed <= self.tokens_per_minute
    }

    /// Wait for token capacity
    pub async fn wait_for_token_capacity(&self, needed: u32) {
        while !self.has_token_capacity(needed).await {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

/// Guard returned when rate limit permission is acquired
pub struct RateLimitGuard {
    _permit: Option<SemaphorePermit<'static>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(5, 1000);

        // Should be able to make 5 requests immediately
        for _ in 0..5 {
            let _guard = limiter.acquire().await;
        }
    }

    #[tokio::test]
    async fn test_token_recording() {
        let limiter = RateLimiter::new(60, 1000);

        limiter.record_tokens(100).await;
        limiter.record_tokens(200).await;

        let usage = limiter.current_token_usage().await;
        assert_eq!(usage, 300);
    }
}
