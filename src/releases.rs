use log::{debug, warn};
use serde::Deserialize;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/SkuldNorniern/lamina/releases/latest";
const LATEST_RELEASE_URL: &str = "https://github.com/SkuldNorniern/lamina/releases/latest";
const CACHE_LIFETIME: Duration = Duration::from_secs(60 * 60);

static RELEASE_CACHE: OnceLock<Mutex<Option<CachedRelease>>> = OnceLock::new();

#[derive(Clone, Debug)]
struct CachedRelease {
    fetched_at: Instant,
    release: GithubRelease,
}

#[derive(Clone, Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    tarball_url: String,
    zipball_url: String,
    assets: Vec<GithubAsset>,
}

#[derive(Clone, Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

/// Render a compact homepage summary from the latest GitHub Release.
///
/// The server refreshes this at most once per hour. If GitHub is temporarily
/// unavailable, a previously fetched release remains usable. The final fallback
/// keeps the page honest without delaying or breaking the homepage.
pub async fn homepage_release_summary() -> String {
    match latest_release().await {
        Some(release) => render_release_summary(&release),
        None => render_unavailable_summary(),
    }
}

async fn latest_release() -> Option<GithubRelease> {
    if let Some(release) = cached_release(false) {
        return Some(release);
    }

    match fetch_latest_release().await {
        Ok(release) => {
            let cache = RELEASE_CACHE.get_or_init(|| Mutex::new(None));
            let mut cache = cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *cache = Some(CachedRelease {
                fetched_at: Instant::now(),
                release: release.clone(),
            });
            Some(release)
        }
        Err(error) => {
            warn!("Unable to refresh latest Lamina release: {error}");
            cached_release(true)
        }
    }
}

fn cached_release(allow_stale: bool) -> Option<GithubRelease> {
    let cache = RELEASE_CACHE.get_or_init(|| Mutex::new(None));
    let cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let cached = cache.as_ref()?;

    if allow_stale || cached.fetched_at.elapsed() < CACHE_LIFETIME {
        Some(cached.release.clone())
    } else {
        None
    }
}

async fn fetch_latest_release() -> Result<GithubRelease, reqwest::Error> {
    debug!("Refreshing latest Lamina GitHub Release");

    reqwest::Client::new()
        .get(LATEST_RELEASE_API)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header(reqwest::header::USER_AGENT, "lamina-web")
        .timeout(Duration::from_secs(3))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

fn render_release_summary(release: &GithubRelease) -> String {
    let mut sdk_links = String::new();
    for asset in release
        .assets
        .iter()
        .filter(|asset| stable_c_sdk_label(&asset.name).is_some())
    {
        let label = stable_c_sdk_label(&asset.name).expect("filtered stable C SDK asset");
        sdk_links.push_str(&format!(
            "<a href=\"{}\">C SDK: {}</a>",
            escape_html(&asset.browser_download_url),
            escape_html(label),
        ));
    }

    format!(
        "<div class=\"lmn-hero-release\" aria-label=\"Latest Lamina release\">\
            <p><span>Latest</span><a href=\"{}\" target=\"_blank\" rel=\"noopener\">{}</a></p>\
            <div class=\"lmn-hero-release-links\">\
                <a href=\"{}\">Source .tar.gz</a>\
                <a href=\"{}\">Source .zip</a>\
                {}\
            </div>\
        </div>",
        escape_html(&release.html_url),
        escape_html(&release.tag_name),
        escape_html(&release.tarball_url),
        escape_html(&release.zipball_url),
        sdk_links_or_preview(&sdk_links),
    )
}

fn sdk_links_or_preview(sdk_links: &str) -> String {
    if sdk_links.is_empty() {
        "<a href=\"/docs/c-bindings/install\">C SDK: source preview</a>".to_string()
    } else {
        sdk_links.to_string()
    }
}

fn render_unavailable_summary() -> String {
    format!(
        "<div class=\"lmn-hero-release\" aria-label=\"Lamina releases\">\
            <p><span>Release</span><a href=\"{LATEST_RELEASE_URL}\" target=\"_blank\" rel=\"noopener\">Check GitHub</a></p>\
            <div class=\"lmn-hero-release-links\">\
                <a href=\"/docs/c-bindings/install\">C SDK: source preview</a>\
            </div>\
        </div>",
    )
}

fn stable_c_sdk_label(filename: &str) -> Option<&'static str> {
    if !filename.starts_with("lamina-c-")
        || filename.contains("-nightly-")
        || !(filename.ends_with(".tar.gz") || filename.ends_with(".zip"))
    {
        return None;
    }

    if filename.contains("-x86_64-linux.") {
        Some("Linux x86_64")
    } else if filename.contains("-x86_64-windows.") {
        Some("Windows x86_64")
    } else if filename.contains("-aarch64-macos.") {
        Some("macOS Apple Silicon")
    } else {
        None
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_tested_stable_archives_only() {
        assert_eq!(
            stable_c_sdk_label("lamina-c-0.1.0-x86_64-linux.tar.gz"),
            Some("Linux x86_64")
        );
        assert_eq!(
            stable_c_sdk_label("lamina-c-0.1.0-x86_64-windows.zip"),
            Some("Windows x86_64")
        );
        assert_eq!(
            stable_c_sdk_label("lamina-c-0.1.0-nightly-20260601-x86_64-linux.tar.gz"),
            None
        );
        assert_eq!(
            stable_c_sdk_label("lamina-c-0.1.0-riscv64-linux.tar.gz"),
            None
        );
    }

    #[test]
    fn source_preview_release_does_not_render_fake_downloads() {
        let release = GithubRelease {
            tag_name: "v0.0.10".to_string(),
            html_url: "https://example.com/releases/v0.0.10".to_string(),
            tarball_url: "https://example.com/releases/v0.0.10.tar.gz".to_string(),
            zipball_url: "https://example.com/releases/v0.0.10.zip".to_string(),
            assets: Vec::new(),
        };

        let rendered = render_release_summary(&release);

        assert!(rendered.contains("v0.0.10"));
        assert!(rendered.contains("C SDK: source preview"));
        assert!(!rendered.contains("C SDK: Linux x86_64"));
    }
}
