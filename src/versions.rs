use log::{debug, error, warn};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const REFRESH_INTERVAL: Duration = Duration::from_secs(30 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

struct Component {
    key: &'static str,
    repo: &'static str,
}

/// The only place a component is tied to the repository its version comes from.
const COMPONENTS: [Component; 4] = [
    Component {
        key: "lamina_version",
        repo: "SkuldNorniern/lamina",
    },
    Component {
        key: "ras_version",
        repo: "Project-Lamina/ras",
    },
    Component {
        key: "weld_version",
        repo: "Project-Lamina/weld",
    },
    Component {
        key: "camina_version",
        repo: "SkuldNorniern/camina",
    },
];

static VERSIONS: OnceLock<Mutex<HashMap<&'static str, String>>> = OnceLock::new();

#[derive(Deserialize)]
struct GithubTag {
    name: String,
}

fn store() -> &'static Mutex<HashMap<&'static str, String>> {
    VERSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Every component key, carrying the latest release tag or an empty string.
///
/// An empty string is what a template tests with `{{#if key}}`, so a component
/// whose version has never been fetched renders no number at all.
pub fn template_variables() -> HashMap<String, String> {
    let cached = store().lock().unwrap_or_else(|p| p.into_inner());
    COMPONENTS
        .iter()
        .map(|component| {
            let version = cached.get(component.key).cloned().unwrap_or_default();
            (component.key.to_string(), version)
        })
        .collect()
}

/// Refresh on a timer so no request ever waits on GitHub.
pub fn spawn_refresh_task() {
    tokio::spawn(async move {
        loop {
            if let Err(e) = tokio::spawn(refresh_versions()).await {
                error!("Version refresh failed: {e}. Retrying after the usual interval");
            }
            tokio::time::sleep(REFRESH_INTERVAL).await;
        }
    });
}

/// A failed or rate-limited fetch keeps whatever is already cached.
async fn refresh_versions() {
    let client = reqwest::Client::new();

    for component in COMPONENTS.iter() {
        match fetch_latest_tag(&client, component.repo).await {
            Ok(Some(version)) => {
                let mut cached = store().lock().unwrap_or_else(|p| p.into_inner());
                debug!("{} is at {version}", component.repo);
                cached.insert(component.key, version);
            }
            Ok(None) => {
                warn!("{} has no tag that looks like a release", component.repo);
            }
            Err(error) => {
                warn!("Keeping the cached version for {}: {error}", component.repo);
            }
        }
    }
}

async fn fetch_latest_tag(
    client: &reqwest::Client,
    repo: &str,
) -> Result<Option<String>, reqwest::Error> {
    let tags: Vec<GithubTag> = client
        .get(format!(
            "https://api.github.com/repos/{repo}/tags?per_page=100"
        ))
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header(reqwest::header::USER_AGENT, "lamina-web")
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(latest_release_version(&tags))
}

/// Highest `MAJOR.MINOR.PATCH` tag. Rolling names such as `latest` and
/// prerelease suffixes carry no release version, so they are skipped.
fn latest_release_version(tags: &[GithubTag]) -> Option<String> {
    tags.iter()
        .filter_map(|tag| parse_release_tag(&tag.name))
        .max()
        .map(|(major, minor, patch)| format!("{major}.{minor}.{patch}"))
}

fn parse_release_tag(name: &str) -> Option<(u64, u64, u64)> {
    let mut parts = name.strip_prefix('v').unwrap_or(name).split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAPTURED_LAMINA_TAGS: &str = r#"[
        {"name":"v0.1.0","commit":{"sha":"bf19d28"}},
        {"name":"v0.0.10","commit":{"sha":"67668f6"}},
        {"name":"v0.0.9","commit":{"sha":"74f62b9"}},
        {"name":"v0.0.2","commit":{"sha":"39d457a"}},
        {"name":"latest","commit":{"sha":"bf19d28"}}
    ]"#;

    const CAPTURED_WELD_TAGS: &str = r#"[
        {"name":"v0.1.2"},
        {"name":"v0.1.1"},
        {"name":"v0.1.0"}
    ]"#;

    fn parse(json: &str) -> Vec<GithubTag> {
        serde_json::from_str(json).expect("captured GitHub tag listing")
    }

    #[test]
    fn picks_the_highest_release_tag_from_a_captured_listing() {
        assert_eq!(
            latest_release_version(&parse(CAPTURED_LAMINA_TAGS)),
            Some("0.1.0".to_string())
        );
        assert_eq!(
            latest_release_version(&parse(CAPTURED_WELD_TAGS)),
            Some("0.1.2".to_string())
        );
    }

    #[test]
    fn orders_numerically_rather_than_lexically() {
        let tags = parse(r#"[{"name":"v0.0.9"},{"name":"v0.0.10"}]"#);
        assert_eq!(latest_release_version(&tags), Some("0.0.10".to_string()));
    }

    #[test]
    fn a_repository_with_no_release_tag_yields_nothing() {
        let tags = parse(r#"[{"name":"latest"},{"name":"nightly"},{"name":"v0.2.0-rc1"}]"#);
        assert_eq!(latest_release_version(&tags), None);
    }

    #[test]
    fn every_component_is_a_template_variable_even_before_a_fetch() {
        let variables = template_variables();
        for component in COMPONENTS.iter() {
            assert!(variables.contains_key(component.key));
        }
    }
}
