//! Rate limiter implementation using sliding window for both RPM and TPM

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const WINDOW_SECS: u64 = 60;

/// Rate limiter using sliding window for both requests-per-minute and tokens-per-minute
pub struct RateLimiter {
    requests_per_minute: u32,
    tokens_per_minute: u32,
    last_requests: Arc<Mutex<VecDeque<Instant>>>,
    token_usage: Arc<Mutex<VecDeque<(Instant, u32)>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(requests_per_minute: u32, tokens_per_minute: u32) -> Self {
        Self {
            requests_per_minute,
            tokens_per_minute,
            last_requests: Arc::new(Mutex::new(VecDeque::new())),
            token_usage: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Acquire permission to make a request (no token estimate — RPM only).
    pub async fn acquire(&self) -> RateLimitGuard {
        self.acquire_with_tokens(0).await
    }

    /// Acquire permission to make a request, enforcing both RPM and TPM.
    ///
    /// `estimated_tokens` is a conservative guess of how many tokens the
    /// upcoming request will consume (input + output). Pass 0 to skip the
    /// TPM check.
    pub async fn acquire_with_tokens(&self, estimated_tokens: u32) -> RateLimitGuard {
        loop {
            // --- RPM check ---
            if let Some(wait) = self.check_request_limit().await {
                tracing::debug!("RPM limit reached, waiting {:.1}s", wait.as_secs_f64());
                tokio::time::sleep(wait).await;
                continue;
            }

            // --- TPM check ---
            if estimated_tokens > 0 {
                if let Some(wait) = self.check_token_limit(estimated_tokens).await {
                    tracing::debug!(
                        "TPM limit reached (est {} tokens), waiting {:.1}s",
                        estimated_tokens,
                        wait.as_secs_f64()
                    );
                    tokio::time::sleep(wait).await;
                    continue;
                }
            }

            // Record the request timestamp
            let mut last = self.last_requests.lock().await;
            last.push_back(Instant::now());

            return RateLimitGuard { _private: () };
        }
    }

    /// Check RPM sliding window. Returns wait duration if at capacity.
    async fn check_request_limit(&self) -> Option<Duration> {
        let mut last = self.last_requests.lock().await;
        let now = Instant::now();
        let window = Duration::from_secs(WINDOW_SECS);

        // Evict entries older than the window
        while let Some(&front) = last.front() {
            if now.duration_since(front) > window {
                last.pop_front();
            } else {
                break;
            }
        }

        if last.len() >= self.requests_per_minute as usize {
            if let Some(&oldest) = last.front() {
                let elapsed = now.duration_since(oldest);
                if elapsed < window {
                    return Some(window - elapsed + Duration::from_millis(100));
                }
            }
        }

        None
    }

    /// Check TPM sliding window. Returns wait duration if adding
    /// `estimated_tokens` would exceed the budget.
    async fn check_token_limit(&self, estimated_tokens: u32) -> Option<Duration> {
        let mut usage = self.token_usage.lock().await;
        let now = Instant::now();
        let window = Duration::from_secs(WINDOW_SECS);

        // Evict entries older than the window
        while let Some(&(time, _)) = usage.front() {
            if now.duration_since(time) > window {
                usage.pop_front();
            } else {
                break;
            }
        }

        let current: u32 = usage.iter().map(|(_, t)| t).sum();

        if current + estimated_tokens <= self.tokens_per_minute {
            return None; // Enough capacity
        }

        // Calculate how long to wait: walk entries oldest-first until
        // enough tokens would expire to make room.
        let excess = (current + estimated_tokens).saturating_sub(self.tokens_per_minute);
        let mut freed = 0u32;
        for &(time, tokens) in usage.iter() {
            freed += tokens;
            if freed >= excess {
                let expiry = time + window;
                return if expiry > now {
                    Some(expiry - now + Duration::from_millis(100))
                } else {
                    Some(Duration::from_millis(100))
                };
            }
        }

        // Need to wait for the full window to rotate
        Some(Duration::from_secs(WINDOW_SECS + 1))
    }

    /// Record actual token usage after a response is received.
    pub async fn record_tokens(&self, tokens: u32) {
        let mut usage = self.token_usage.lock().await;
        let now = Instant::now();
        let window = Duration::from_secs(WINDOW_SECS);

        // Evict old entries
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
        let window = Duration::from_secs(WINDOW_SECS);

        while let Some(&(time, _)) = usage.front() {
            if now.duration_since(time) > window {
                usage.pop_front();
            } else {
                break;
            }
        }

        usage.iter().map(|(_, t)| t).sum()
    }
}

/// Guard returned when rate limit permission is acquired
pub struct RateLimitGuard {
    _private: (),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(5, 100_000);

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

    #[tokio::test]
    async fn test_tpm_blocks_when_over_budget() {
        let limiter = RateLimiter::new(60, 500);

        // Record 400 tokens of recent usage
        limiter.record_tokens(400).await;

        // Requesting 200 more should exceed the 500 TPM budget
        let wait = limiter.check_token_limit(200).await;
        assert!(wait.is_some(), "should have been asked to wait");
    }

    #[tokio::test]
    async fn test_tpm_allows_when_under_budget() {
        let limiter = RateLimiter::new(60, 500);

        limiter.record_tokens(100).await;

        // 100 used + 200 estimated = 300 < 500
        let wait = limiter.check_token_limit(200).await;
        assert!(wait.is_none(), "should have capacity");
    }
}
