use std::time::{Duration, SystemTime};

/// 调度 jitter 来源。
pub trait JitterSource {
    /// 返回不大于 `upper_bound` 的随机延迟。
    fn jitter(&mut self, upper_bound: Duration) -> Duration;
}

/// 基于进程内状态的轻量随机 jitter 来源。
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SystemJitter {
    state: u64,
}

impl SystemJitter {
    /// 用固定种子构造 jitter 来源。
    #[must_use]
    pub const fn from_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    const fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let first = (self.state ^ (self.state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        let second = (first ^ (first >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        second ^ (second >> 31)
    }
}

impl Default for SystemJitter {
    fn default() -> Self {
        Self::from_seed(seed_from_time())
    }
}

impl JitterSource for SystemJitter {
    fn jitter(&mut self, upper_bound: Duration) -> Duration {
        if upper_bound.is_zero() {
            return Duration::ZERO;
        }
        let modulo = upper_bound.as_nanos().saturating_add(1);
        let nanos = u128::from(self.next_u64()) % modulo;
        let bounded_nanos = u64::try_from(nanos).map_or(u64::MAX, std::convert::identity);
        Duration::from_nanos(bounded_nanos)
    }
}

fn seed_from_time() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() ^ u64::from(duration.subsec_nanos()).rotate_left(32),
        Err(_error) => 0xA5A5_A5A5_5A5A_5A5A,
    }
}
