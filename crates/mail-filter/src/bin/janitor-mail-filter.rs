use std::error::Error;
use std::fs::File;
use std::io::{self, Read};
use std::process;
use log::debug;

use clap::{Arg, App};
use log::{error,info};
use reqwest::blocking::{Client, RequestBuilder};

fn refresh_merge_proposal(api_url: &str, merge_proposal_url: &str) -> Result<(), String> {
    let client = Client::new();
    let res = client.post(api_url)
        .json(&serde_json::json!({"url": merge_proposal_url}))
        .send().map_err(|e| e.to_string())?;

    match res.status().as_u16() {
        200 | 202 => Ok(()),
        status => Err(format!("error {} triggering refresh for {}", status, api_url).into()),
    }
}

fn main() {
    let matches = App::new("Email Parser")
        .arg(Arg::with_name("refresh-url")
            .help("URL to submit requests to.")
            .default_value("https://janitor.debian.net/api/refresh-proposal-status")
            .takes_value(true))
        .arg(Arg::with_name("input")
            .help("Path to read mail from.")
            .default_value("/dev/stdin")
            .takes_value(true))
        .get_matches();

    let refresh_url = matches.value_of("refresh-url").unwrap();
    let input = matches.value_of("input").unwrap();

    let f = File::open(input).unwrap();

    match janitor_mail_filter::parse_email(f) {
        Some(merge_proposal_url) => {
            info!("Found merge proposal URL: {}", merge_proposal_url);
            match refresh_merge_proposal(refresh_url, &merge_proposal_url) {
                Ok(()) => process::exit(0),
                Err(e) => {
                    error!("Error: {}", e);
                    process::exit(1);
                },
            }
        },
        None => {
            debug!("No merge proposal URL found.");
            process::exit(0);
        },
    }
}
