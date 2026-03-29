use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = "org-cli.toml";

// ==================== Top-level config ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub site: SiteConfig,
    pub blog: BlogConfig,
    pub scrub: ScrubConfig,
    pub images: ImagesConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            site: SiteConfig::default(),
            blog: BlogConfig::default(),
            scrub: ScrubConfig::default(),
            images: ImagesConfig::default(),
        }
    }
}

impl Config {
    /// Load from `org-cli.toml` in `dir`, or return defaults if the file
    /// does not exist.
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join(CONFIG_FILE);
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        toml::from_str(&raw)
            .with_context(|| format!("Failed to parse {}", path.display()))
    }

    /// Resolve `output_dir` relative to `base`, expanding `~`.
    pub fn resolved_output(&self, base: &Path) -> PathBuf {
        resolve_path(&self.site.output_dir, base)
    }
}

fn resolve_path(p: &str, base: &Path) -> PathBuf {
    if p.starts_with('~') {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(&p[2..]);
        }
    }
    let pb = PathBuf::from(p);
    if pb.is_absolute() {
        pb
    } else {
        base.join(pb)
    }
}

// ==================== [site] ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SiteConfig {
    /// Default site title used as `<title>` fallback.
    pub title: String,
    pub base_url: String,
    /// Directory containing .org source files (relative to config file).
    pub source_dir: String,
    /// Directory to write HTML output into.
    pub output_dir: String,
    /// Extra directories copied verbatim into output.
    pub static_dirs: Vec<String>,
    /// Absolute path prefix to strip from href/src attributes in HTML output.
    pub strip_path_prefix: String,
    /// Path to the HTML file served for private pages.
    pub private_placeholder: String,
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            title: String::new(),
            base_url: String::new(),
            source_dir: ".".into(),
            output_dir: "public_html".into(),
            static_dirs: vec!["static".into()],
            strip_path_prefix: String::new(),
            private_placeholder: "private.html".into(),
        }
    }
}

// ==================== [blog] ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BlogConfig {
    pub enabled: bool,
    /// Name of the generated blog index file (written into source_dir).
    pub index_file: String,
    /// Seed for the deterministic random-post selection.
    pub nav_random_seed: u64,
}

impl Default for BlogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            index_file: "blog.org".into(),
            nav_random_seed: 42,
        }
    }
}

// ==================== [scrub] ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScrubConfig {
    pub enabled: bool,
    /// Path to scrub.toml containing the substitution rules.
    pub rules_file: String,
    /// Output HTML filenames that should NOT be scrubbed.
    pub skip_files: Vec<String>,
}

impl Default for ScrubConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rules_file: "scrub.toml".into(),
            skip_files: Vec::new(),
        }
    }
}

/// A single real→fake substitution rule loaded from `scrub.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrubRule {
    pub category: ScrubCategory,
    pub real: String,
    pub fake: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScrubCategory {
    Address,
    Town,
    Zip,
    Email,
    Phone,
    Carrier,
    Other,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScrubRules {
    #[serde(default, rename = "rule")]
    pub rules: Vec<ScrubRule>,
}

impl ScrubRules {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        toml::from_str(&raw)
            .with_context(|| format!("Failed to parse {}", path.display()))
    }
}

// ==================== [images] ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ImagesConfig {
    pub enabled: bool,
    pub max_width: u32,
    pub max_height: u32,
    pub quality: u8,
    pub greyscale: bool,
    pub grain: bool,
}

impl Default for ImagesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_width: 1280,
            max_height: 720,
            quality: 80,
            greyscale: true,
            grain: true,
        }
    }
}
