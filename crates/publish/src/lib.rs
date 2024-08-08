use chrono::{DateTime, Utc};
use reqwest::header::HeaderMap;

//pub mod publish_one;

pub fn calculate_next_try_time(finish_time: DateTime<Utc>, attempt_count: usize) -> DateTime<Utc> {
    if attempt_count == 0 {
        finish_time
    } else {
        let delta = chrono::Duration::hours(2usize.pow(attempt_count as u32).min(7 * 24) as i64);

        finish_time + delta
    }
}

#[derive(Debug)]
pub enum DebdiffError {
    Http(reqwest::Error),
    MissingRun(String),
    Unavailable(String),
}

impl From<reqwest::Error> for DebdiffError {
    fn from(e: reqwest::Error) -> Self {
        DebdiffError::Http(e)
    }
}

impl std::fmt::Display for DebdiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DebdiffError::Http(e) => write!(f, "HTTP error: {}", e),
            DebdiffError::MissingRun(e) => write!(f, "Missing run: {}", e),
            DebdiffError::Unavailable(e) => write!(f, "Unavailable: {}", e),
        }
    }
}

impl std::error::Error for DebdiffError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DebdiffError::Http(e) => Some(e),
            _ => None,
        }
    }
}

pub async fn get_debdiff(differ_url: &url::Url, unchanged_id: &str, log_id: &str) -> Result<Vec<u8>, DebdiffError> {
    let debdiff_url = differ_url.join(&format!("/debdiff/{}/{}?filter_boring=1", unchanged_id, log_id)).unwrap();

    let mut headers = HeaderMap::new();
    headers.insert("Accept", "text/plain".parse().unwrap());

    let client = reqwest::Client::new();
    let response = client.get(debdiff_url).headers(headers).send().await?;

    match response.status() {
        reqwest::StatusCode::OK => Ok(response.bytes().await?.to_vec()),
        reqwest::StatusCode::NOT_FOUND => {
            let run_id = response.headers().get("unavailable_run_id").unwrap().to_str().unwrap();
            Err(DebdiffError::MissingRun(run_id.to_string()))
        },
        reqwest::StatusCode::BAD_REQUEST | reqwest::StatusCode::INTERNAL_SERVER_ERROR | reqwest::StatusCode::BAD_GATEWAY | reqwest::StatusCode::SERVICE_UNAVAILABLE | reqwest::StatusCode::GATEWAY_TIMEOUT => {
            Err(DebdiffError::Unavailable(response.text().await.unwrap()))
        },
        _e => Err(DebdiffError::Http(response.error_for_status().unwrap_err()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_calculate_next_try_time() {
        let finish_time = Utc::now();
        let attempt_count = 0;
        let next_try_time = calculate_next_try_time(finish_time, attempt_count);
        assert_eq!(finish_time, next_try_time);

        let attempt_count = 1;
        let next_try_time = calculate_next_try_time(finish_time, attempt_count);
        assert_eq!(finish_time + chrono::Duration::hours(2), next_try_time);

        let attempt_count = 2;
        let next_try_time = calculate_next_try_time(finish_time, attempt_count);
        assert_eq!(finish_time + chrono::Duration::hours(4), next_try_time);

        let attempt_count = 3;
        let next_try_time = calculate_next_try_time(finish_time, attempt_count);
        assert_eq!(finish_time + chrono::Duration::hours(8), next_try_time);

        // Verify that the maximum delay is 7 days
        let attempt_count = 10;
        let next_try_time = calculate_next_try_time(finish_time, attempt_count);
        assert_eq!(finish_time + chrono::Duration::days(7), next_try_time);
    }
}
