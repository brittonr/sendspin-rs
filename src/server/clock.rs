// ABOUTME: Server-side monotonic clock
// ABOUTME: Provides stable timestamps for audio synchronization

use std::time::Instant;

/// Server clock for generating timestamps
///
/// The server uses a monotonic clock starting from when the server was created.
/// All timestamps are in microseconds from this start point.
#[derive(Debug)]
pub struct ServerClock {
    /// When the server started
    start: Instant,
}

impl ServerClock {
    /// Create a new server clock starting now
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Get current server time in microseconds
    #[inline]
    pub fn now_micros(&self) -> i64 {
        self.start.elapsed().as_micros() as i64
    }

    /// Get the server start instant (for computing deltas)
    pub fn start(&self) -> Instant {
        self.start
    }

    /// Convert server microseconds to duration from start
    pub fn micros_to_duration(&self, micros: i64) -> std::time::Duration {
        std::time::Duration::from_micros(micros as u64)
    }
}

impl Default for ServerClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_clock_monotonic() {
        let clock = ServerClock::new();
        let t1 = clock.now_micros();
        sleep(Duration::from_millis(10));
        let t2 = clock.now_micros();

        assert!(t2 > t1, "Clock should be monotonically increasing");
        assert!(t2 - t1 >= 10_000, "At least 10ms should have passed");
    }
}
