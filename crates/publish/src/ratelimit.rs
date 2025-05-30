use breezyshim::forge::MergeProposalStatus;
use std::collections::HashMap;
use url::Url;

type Bucket = MergeProposalStatus;

/// Returned when a rate limit is hit
#[derive(Debug, Clone)]
pub enum RateLimited {
    NotYetDetermined {
        /// The bucket that was rate limited
        bucket: Bucket,
    },

    /// Rate limit hit
    LimitReached {
        /// The bucket that was rate limited
        bucket: Bucket,

        /// Current number of merge proposals in the bucket
        current: usize,

        /// Maximum number of merge proposals allowed in the bucket
        max_open: usize,
    },
}

impl std::fmt::Display for RateLimited {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RateLimited::NotYetDetermined { bucket } => {
                write!(f, "Rate limited on bucket: {} not yet determined", bucket)
            }
            RateLimited::LimitReached {
                bucket,
                current,
                max_open,
            } => write!(
                f,
                "Rate limited on bucket: {} with {} merge proposals, max allowed: {}",
                bucket, current, max_open
            ),
        }
    }
}

pub trait RateLimiter {
    /// Set the current number of merge proposals per bucket
    fn set_mps_per_bucket(&mut self, mps_per_bucket: HashMap<Bucket, HashMap<Url, usize>>);

    /// Check if the bucket is allowed to have more merge proposals
    fn check_allowed(&self, bucket: Bucket) -> Option<RateLimited>;

    /// Increment the number of merge proposals in the bucket
    fn inc(&mut self, bucket: &Bucket);

    /// Get the current stats for the rate limiter
    fn get_stats(&self) -> HashMap<Bucket, (usize, Option<usize>)>;
}

pub struct NonRateLimiter;

impl RateLimiter for NonRateLimiter {
    fn set_mps_per_bucket(&mut self, _mps_per_bucket: HashMap<Bucket, HashMap<Url, usize>>) {}

    fn check_allowed(&self, _bucket: Bucket) -> Option<RateLimited> {
        None
    }

    fn inc(&mut self, _bucket: &Bucket) {}

    fn get_stats(&self) -> HashMap<Bucket, (usize, Option<usize>)> {
        HashMap::new()
    }
}

pub struct FixedRateLimiter {
    open_mps_per_bucket: Option<HashMap<Bucket, usize>>,
    max_mps_per_bucket: Option<usize>,
}

impl FixedRateLimiter {
    fn new(max_mps_per_bucket: Option<usize>) -> Self {
        Self {
            open_mps_per_bucket: Some(HashMap::new()),
            max_mps_per_bucket,
        }
    }

    fn set_mps_per_bucket(&mut self, mps_per_bucket: HashMap<Bucket, HashMap<Url, usize>>) {
        self.open_mps_per_bucket = mps_per_bucket.drain(MergeProposalStatus::Open);
    }

    fn check_allowed(self, bucket: MergeProposalStatus) -> Option<RateLimited> {
        if self.max_mps_per_bucket.is_none() {
            return None;
        }
        if self.open_mps_per_bucket.is_none() {
            // Be conservative
            return Some(RateLimited::NotYetDetermined { bucket });
        }
        let current = self
            .open_mps_per_bucket
            .as_ref()
            .unwrap()
            .get(&bucket)
            .unwrap_or(&0);
        if *current > self.max_mps_per_bucket.unwrap() {
            Some(RateLimited::LimitReached {
                bucket,
                current: *current,
                max_open: self.max_mps_per_bucket.unwrap(),
            })
        } else {
            None
        }
    }

    fn inc(&mut self, bucket: &Bucket) {
        if self.open_mps_per_bucket.is_none() {
            return;
        }
        self.open_mps_per_bucket
            .as_mut()
            .unwrap()
            .get_mut(bucket)
            .map(|mps| *mps += 1);
    }

    fn get_stats(&self) -> HashMap<Bucket, (usize, Option<usize>)> {
        if let Some(open_mps_per_bucket) = self.open_mps_per_bucket.as_ref() {
            open_mps_per_bucket
                .iter()
                .map(|(bucket, current)| (bucket.clone(), (*current, self.max_mps_per_bucket)))
                .collect()
        } else {
            return HashMap::new();
        }
    }
}

pub struct SlowStartRateLimiter {}

impl SlowStartRateLimiter {
    pub fn new(max_mps_per_bucket: Option<
    def __init__(self, max_mps_per_bucket=None) -> None:
        self.max_mps_per_bucket = max_mps_per_bucket
        self.open_mps_per_bucket: Optional[dict[str, int]] = None
        self.absorbed_mps_per_bucket: Optional[dict[str, int]] = None
    }

    fn get_limit(&self, bucket: &Bucket) -> Option<usize> {
        if let Some(absorbed_mps_per_bucket) = self.absorbed_mps_per_bucket {
            absorbed_mps_per_bucket.get(bucket).unwrap_or(0) + 1
        } else {
            None
        }
    }
}

impl RateLimiter for SlowStartRateLimiter {
    fn check_allowed(&self, bucket: &Bucket) -> Option<RateLimited> {
        if self.open_mps_per_bucket.is_none() || self.absorbed_mps_per_bucket.is_none() {
            // Be conservative
            return RateLimited::NotYetDetermined { bucket };
        }
        let current = self.open_mps_per_bucket.get(bucket).unwrap_or(0);
        if let Some(max_mps_per_bucket) = self.max_mps_per_bucket {
            if current >= self.max_mps_per_bucket {
                return Some(RateLimited::LimitReached { bucket, current, limit });
            }
        }
        let limit = self.get_limit(bucket);
        if let Some(limit) = limit {
            if current >= limit {
                return Some(RateLimited::LimitReached { bucket, current, limit });
            }
        }
    }

    fn inc(&mut self, bucket: &Bucket) {
        if let Some(open_mps_per_bucket) = self.open_mps_per_bucket.as_mut() {
            open_mps_per_bucket.get_mut(bucket).map(|mps| *mps += 1);
        }
    }

    fn set_mps_per_bucket(&mut self, mps_per_bucket: HashMap<Bucket, HashMap<Url, usize>>) {
        self.open_mps_per_bucket = mps_per_bucket.get(MergeProposalStatus::Open).unwrap_or_else(HashMap::new);
        let ms = HashMap::new();
        for status in ["merged", "applied"] {
            for m, c in mps_per_bucket.get(status)
                ms.setdefault(m, 0)
                ms[m] += c
        self.absorbed_mps_per_bucket = ms;
    }

    fn get_stats(&self) -> HashMap<Bucket, (usize, Option<usize>)> {
        if let Some(open_mps_per_bucket) = self.open_mps_per_bucket {
            open_mps_per_bucket.iter()
                .map(|(bucket, current)| (current, self.get_limit(bucket).min(self.max_mps_per_bucket)))
                .collect()
        } else {
            HashMap::new()
        }
    }
}
