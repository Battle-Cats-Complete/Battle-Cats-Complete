use std::time::{SystemTime, UNIX_EPOCH};

use tracing::{debug, warn};

use super::github;
use super::io::cache::{self, CacheSpec};

const TTL_SECS: u64 = 60 * 60 * 6;

struct Cache;

impl CacheSpec for Cache {
    type Data = Vec<(String, String)>;
    const FILE: &'static str = "changelog";
}

pub fn load(owner: &str, repo: &str, current_version: &str) -> Result<Vec<(String, String)>, github::Error> {
    let Some((fetched_at, cached)) = cache::read::<Cache>() else {
        return github::list_releases(owner, repo).map(store);
    };

    if is_fresh(fetched_at) && cached.iter().any(|(version, _)| version == current_version) {
        debug!("Serving {} changelog entries from the cache", cached.len());
        return Ok(cached);
    }

    match github::list_releases(owner, repo) {
        Ok(releases) => Ok(store(releases)),
        Err(err) => {
            warn!("Serving the stale cached changelog, GitHub was unreachable: {}", err);
            Ok(cached)
        }
    }
}

fn store(releases: Vec<github::Release>) -> Vec<(String, String)> {
    let entries: Vec<(String, String)> = releases
        .into_iter()
        .filter(github::Release::is_versioned)
        .map(|release| {
            let version = release.version().to_string();
            let notes = release
                .body
                .filter(|body| !body.trim().is_empty())
                .unwrap_or_else(|| "No notes.".to_string());

            (version, notes)
        })
        .collect();

    cache::write::<Cache>(now(), &entries);
    entries
}

fn is_fresh(fetched_at: u64) -> bool {
    now().saturating_sub(fetched_at) < TTL_SECS
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}
