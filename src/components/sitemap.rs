use chrono::{DateTime, Utc};
use log::{debug, info};

/// Sitemap entry representing a URL in the sitemap
#[derive(Debug, Clone)]
pub struct SitemapEntry {
    pub url: String,
    pub last_modified: DateTime<Utc>,
    pub change_frequency: ChangeFrequency,
    pub priority: f32,
}

/// Change frequency for sitemap entries
#[derive(Debug, Clone)]
pub enum ChangeFrequency {
    Daily,
    Weekly,
}

impl ChangeFrequency {
    fn as_str(&self) -> &'static str {
        match self {
            ChangeFrequency::Daily => "daily",
            ChangeFrequency::Weekly => "weekly",
        }
    }
}

/// Sitemap generator for the website
pub struct SitemapGenerator {
    base_url: String,
}

impl SitemapGenerator {
    /// Create a new sitemap generator with the given base URL
    pub fn new(base_url: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Generate the complete sitemap XML.
    pub fn generate_sitemap(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        info!("Generating sitemap for base URL: {}", self.base_url);

        let mut entries = Vec::new();

        // Add static pages
        self.add_static_pages(&mut entries);

        // Generate XML
        let xml = self.generate_xml(&entries)?;

        info!("Generated sitemap with {} entries", entries.len());
        Ok(xml)
    }

    /// Add static pages to the sitemap
    fn add_static_pages(&self, entries: &mut Vec<SitemapEntry>) {
        debug!("Adding static pages to sitemap");

        let now = Utc::now();

        // Homepage
        entries.push(SitemapEntry {
            url: format!("{}/", self.base_url),
            last_modified: now,
            change_frequency: ChangeFrequency::Daily,
            priority: 1.0,
        });

        // Official documentation
        entries.push(SitemapEntry {
            url: format!("{}/docs", self.base_url),
            last_modified: now,
            change_frequency: ChangeFrequency::Weekly,
            priority: 0.9,
        });

        for page in [
            "getting-started",
            "ir",
            "cli",
            "rust",
            "irbuilder",
            "targets",
            "c-bindings",
            "c-bindings/install",
            "c-bindings/usage",
        ] {
            entries.push(SitemapEntry {
                url: format!("{}/docs/{}", self.base_url, page),
                last_modified: now,
                change_frequency: ChangeFrequency::Weekly,
                priority: 0.8,
            });
        }
    }

    /// Generate XML from sitemap entries
    fn generate_xml(
        &self,
        entries: &[SitemapEntry],
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut xml = String::new();

        // XML header
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

        // Add each entry
        for entry in entries {
            xml.push_str("  <url>\n");
            xml.push_str(&format!("    <loc>{}</loc>\n", entry.url));
            xml.push_str(&format!(
                "    <lastmod>{}</lastmod>\n",
                entry.last_modified.format("%Y-%m-%d")
            ));
            xml.push_str(&format!(
                "    <changefreq>{}</changefreq>\n",
                entry.change_frequency.as_str()
            ));
            xml.push_str(&format!("    <priority>{:.1}</priority>\n", entry.priority));
            xml.push_str("  </url>\n");
        }

        xml.push_str("</urlset>");

        Ok(xml)
    }
}

/// Generate sitemap XML for the website
pub fn generate_sitemap_xml(
    base_url: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let generator = SitemapGenerator::new(base_url.to_string());
    generator.generate_sitemap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_minimal_canonical_sitemap() {
        let xml = generate_sitemap_xml("https://lamina.sh/").expect("sitemap generation");

        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">"));
        assert!(xml.contains("<loc>https://lamina.sh/</loc>"));
        assert!(xml.contains("<loc>https://lamina.sh/docs/c-bindings/install</loc>"));
        assert!(!xml.contains("lamina.sh//"));
        assert!(xml.contains("<lastmod>"));
        assert!(xml.contains("<changefreq>daily</changefreq>"));
        assert!(xml.contains("<changefreq>weekly</changefreq>"));
        assert!(xml.contains("<priority>1.0</priority>"));
        assert!(xml.contains("<priority>0.8</priority>"));
    }
}
