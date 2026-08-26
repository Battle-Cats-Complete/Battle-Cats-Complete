use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::blocking::{Client, Response};
use reqwest::StatusCode;
use serde::Deserialize;
use tracing::{debug, warn};

const API_ROOT: &str = "https://api.github.com";
const AGENT: &str = concat!("BattleCatsComplete/", env!("CARGO_PKG_VERSION"));
const PER_PAGE: usize = 100;
const MAX_PAGES: usize = 200;
const TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    RateLimited,
    InvalidUrl,
    Network,
    Malformed,
}

#[derive(Clone, Debug)]
pub struct Error {
    pub kind: ErrorKind,
    message: String,
}

impl Error {
    fn new(kind: ErrorKind, message: String) -> Self {
        Self { kind, message }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Asset {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

impl Release {
    pub fn version(&self) -> &str {
        self.tag_name.trim_start_matches('v')
    }

    pub fn is_versioned(&self) -> bool {
        let version = self.version();
        !version.is_empty() && version.chars().all(|c| c.is_ascii_digit() || c == '.')
    }

    pub fn has_asset(&self, name: &str) -> bool {
        self.assets.iter().any(|asset| asset.name == name)
    }
}

pub fn latest_release(owner: &str, repo: &str) -> Result<Option<Release>, Error> {
    let client = client()?;
    let url = format!("{}/repos/{}/{}/releases/latest", API_ROOT, owner, repo);
    let response = send(&client, &url)?;

    if response.status() == StatusCode::NOT_FOUND {
        debug!("{}/{} has no published releases", owner, repo);
        return Ok(None);
    }

    let body = response
        .text()
        .map_err(|err| Error::new(ErrorKind::Network, format!("could not read the latest release: {}", err)))?;

    serde_json::from_str(&body)
        .map(Some)
        .map_err(|err| Error::new(ErrorKind::Malformed, format!("could not parse the latest release: {}", err)))
}

pub fn list_releases(owner: &str, repo: &str) -> Result<Vec<Release>, Error> {
    let client = client()?;
    let url = format!("{}/repos/{}/{}/releases", API_ROOT, owner, repo);
    let mut releases = Vec::new();

    for page in 1..=MAX_PAGES {
        let batch = fetch_page(&client, &url, page)?;
        let is_last = batch.len() < PER_PAGE;
        releases.extend(batch);

        if is_last {
            debug!("Listed {} releases for {}/{} over {} page(s)", releases.len(), owner, repo, page);
            return Ok(releases);
        }
    }

    warn!("Stopped listing {}/{} releases at the {} page ceiling", owner, repo, MAX_PAGES);
    Ok(releases)
}

fn fetch_page(client: &Client, url: &str, page: usize) -> Result<Vec<Release>, Error> {
    let paged = format!("{}?per_page={}&page={}", url, PER_PAGE, page);
    let response = send(client, &paged)?;

    if response.status() == StatusCode::NOT_FOUND {
        return Err(Error::new(ErrorKind::InvalidUrl, format!("GitHub has no release listing at {}", url)));
    }

    let body = response
        .text()
        .map_err(|err| Error::new(ErrorKind::Network, format!("could not read release page {}: {}", page, err)))?;

    serde_json::from_str(&body)
        .map_err(|err| Error::new(ErrorKind::Malformed, format!("could not parse release page {}: {}", page, err)))
}

fn client() -> Result<Client, Error> {
    Client::builder()
        .user_agent(AGENT)
        .timeout(TIMEOUT)
        .build()
        .map_err(|err| Error::new(ErrorKind::Network, format!("could not build the http client: {}", err)))
}

fn send(client: &Client, url: &str) -> Result<Response, Error> {
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .map_err(|err| {
            let kind = if err.is_builder() { ErrorKind::InvalidUrl } else { ErrorKind::Network };
            Error::new(kind, format!("request to GitHub failed: {}", err))
        })?;

    let status = response.status();

    if status.is_success() || status == StatusCode::NOT_FOUND {
        return Ok(response);
    }

    if is_rate_limited(&response) {
        return Err(Error::new(ErrorKind::RateLimited, rate_limit_message(&response)));
    }

    Err(Error::new(ErrorKind::Network, format!("GitHub answered {}", status)))
}

fn is_rate_limited(response: &Response) -> bool {
    if !matches!(response.status(), StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS) {
        return false;
    }

    response
        .headers()
        .get("x-ratelimit-remaining")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|remaining| remaining == "0")
}

fn rate_limit_message(response: &Response) -> String {
    let seconds = response
        .headers()
        .get("x-ratelimit-reset")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .zip(SystemTime::now().duration_since(UNIX_EPOCH).ok())
        .map(|(reset, now)| reset.saturating_sub(now.as_secs()));

    seconds.map_or_else(
        || "GitHub rate limit reached (60 requests an hour without a token)".to_string(),
        |seconds| {
            format!(
                "GitHub rate limit reached (60 requests an hour without a token); it clears in {}m {}s",
                seconds / 60,
                seconds % 60
            )
        },
    )
}
