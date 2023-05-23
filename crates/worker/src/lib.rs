use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use std::net::IpAddr;
use tokio::net::lookup_host;

pub async fn is_gce_instance() -> bool {
    match lookup_host("metadata.google.internal").await {
        Ok(lookup_result) => {
            for addr in lookup_result {
                if let IpAddr::V4(ipv4) = addr.ip() {
                    if ipv4.is_private() {
                        return true;
                    }
                }
            }
            false
        }
        Err(_) => false,
    }
}

pub async fn gce_external_ip() -> Result<Option<String>, reqwest::Error> {
    let url = "http://metadata.google.internal/computeMetadata/v1/instance/network-interfaces/0/access-configs/0/external-ip";
    let mut headers = HeaderMap::new();
    headers.insert("Metadata-Flavor", HeaderValue::from_static("Google"));

    let client = Client::new();
    let resp = client.get(url).headers(headers).send().await?;

    match resp.status().as_u16() {
        200 => Ok(Some(resp.text().await?)),
        404 => Ok(None),
        _ => panic!("Unexpected response status: {}", resp.status()),
    }
}
