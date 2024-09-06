use std::collections::HashMap;

pub struct RateLimitStats {
    pub per_bucket: HashMap<String, usize>,
}

pub trait RateLimiter {
    fn set_mps_per_bucket(&self, mps_per_bucket: &HashMap<String, HashMap<String, usize>>);

    fn check_allowed(&self, bucket: &str) -> bool;

    fn inc(&self, bucket: &str);

    fn get_stats(&self) -> RateLimitStats;
}
