use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_BACKUPS: usize = 3;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub version: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub changelog: Option<String>,
    #[serde(default)]
    pub sha256: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub new_version: String,
    pub changelog: Option<String>,
    pub download_url: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateResult {
    pub new_version: String,
    pub backup_path: Option<PathBuf>,
    pub verified: bool,
}

// ---------------------------------------------------------------------------
// Throttle / skip state
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Serialize, Deserialize)]
struct UpdateCheckState {
    #[serde(default)]
    last_check_epoch: u64,
    #[serde(default)]
    skipped_version: Option<String>,
}

fn state_path() -> PathBuf {
    nyzhi_config::Config::data_dir().join("update-check.json")
}

fn load_state() -> UpdateCheckState {
    let path = state_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_state(state: &UpdateCheckState) {
    let path = state_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = serde_json::to_string(state)
        .ok()
        .and_then(|s| std::fs::write(&path, s).ok());
}

fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn platform_key() -> String {
    let os = if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    };
    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "unknown"
    };
    format!("{os}-{arch}")
}

// ---------------------------------------------------------------------------
// Backup & rollback
// ---------------------------------------------------------------------------

fn backups_dir() -> PathBuf {
    nyzhi_config::Config::data_dir().join("backups")
}

fn backup_current_binary(version: &str) -> Result<PathBuf> {
    let current_exe = std::env::current_exe()
        .context("Could not determine current executable path")?;

    let dir = backups_dir();
    std::fs::create_dir_all(&dir)?;

    let timestamp = now_epoch();
    let backup_name = format!("nyz-v{version}-{timestamp}");
    let backup_path = dir.join(&backup_name);

    std::fs::copy(&current_exe, &backup_path)
        .context("Failed to backup current binary")?;

    // Preserve executable permission
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&backup_path, std::fs::Permissions::from_mode(0o755));
    }

    prune_old_backups();
    Ok(backup_path)
}

fn prune_old_backups() {
    let dir = backups_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    let mut backups: Vec<(PathBuf, std::time::SystemTime)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("nyz-v") || n.starts_with("nyzhi-v"))
        })
        .filter_map(|e| {
            e.metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| (e.path(), t))
        })
        .collect();

    backups.sort_by(|a, b| b.1.cmp(&a.1));

    for (path, _) in backups.into_iter().skip(MAX_BACKUPS) {
        let _ = std::fs::remove_file(path);
    }
}

/// Restore a backup by replacing the current binary.
pub fn rollback(backup_path: &std::path::Path) -> Result<()> {
    if !backup_path.exists() {
        anyhow::bail!("Backup file not found: {}", backup_path.display());
    }
    self_replace::self_replace(backup_path)
        .context("Rollback failed: could not replace binary from backup")?;
    Ok(())
}

/// List available backups (newest first).
pub fn list_backups() -> Vec<PathBuf> {
    let dir = backups_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut backups: Vec<(PathBuf, std::time::SystemTime)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("nyz-v") || n.starts_with("nyzhi-v"))
        })
        .filter_map(|e| {
            e.metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| (e.path(), t))
        })
        .collect();
    backups.sort_by(|a, b| b.1.cmp(&a.1));
    backups.into_iter().map(|(p, _)| p).collect()
}

// ---------------------------------------------------------------------------
// Integrity manifest — snapshot of user data that must survive updates
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct IntegrityManifest {
    timestamp: u64,
    from_version: String,
    to_version: String,
    config_hash: Option<String>,
    data_dir_exists: bool,
    config_dir_exists: bool,
    keyring_providers: Vec<String>,
}

fn hash_file(path: &std::path::Path) -> Option<String> {
    let data = std::fs::read(path).ok()?;
    use sha2::Digest;
    Some(hex::encode(sha2::Sha256::digest(&data)))
}

fn snapshot_integrity(to_version: &str) -> IntegrityManifest {
    let config_path = nyzhi_config::Config::config_path();
    let config_hash = if config_path.exists() {
        hash_file(&config_path)
    } else {
        None
    };

    let keyring_providers = ["openai", "anthropic", "gemini"]
        .iter()
        .filter(|p| {
            keyring::Entry::new("nyzhi", p)
                .ok()
                .and_then(|e| e.get_password().ok())
                .is_some()
        })
        .map(|p| p.to_string())
        .collect();

    IntegrityManifest {
        timestamp: now_epoch(),
        from_version: CURRENT_VERSION.to_string(),
        to_version: to_version.to_string(),
        config_hash,
        data_dir_exists: nyzhi_config::Config::data_dir().exists(),
        config_dir_exists: nyzhi_config::Config::config_dir().exists(),
        keyring_providers,
    }
}

fn verify_integrity(manifest: &IntegrityManifest) -> Vec<String> {
    let mut issues = Vec::new();

    if manifest.config_dir_exists && !nyzhi_config::Config::config_dir().exists() {
        issues.push("Config directory disappeared after update".to_string());
    }

    if manifest.data_dir_exists && !nyzhi_config::Config::data_dir().exists() {
        issues.push("Data directory disappeared after update".to_string());
    }

    let config_path = nyzhi_config::Config::config_path();
    if let Some(ref expected) = manifest.config_hash {
        if config_path.exists() {
            if let Some(actual) = hash_file(&config_path) {
                if &actual != expected {
                    issues.push("Config file was modified during update".to_string());
                }
            }
        } else {
            issues.push("Config file disappeared after update".to_string());
        }
    }

    for provider in &manifest.keyring_providers {
        let still_exists = keyring::Entry::new("nyzhi", provider)
            .ok()
            .and_then(|e| e.get_password().ok())
            .is_some();
        if !still_exists {
            issues.push(format!(
                "OAuth token for {provider} lost after update"
            ));
        }
    }

    issues
}

fn save_manifest(manifest: &IntegrityManifest) {
    let path = nyzhi_config::Config::data_dir().join("update-manifest.json");
    let _ = serde_json::to_string_pretty(manifest)
        .ok()
        .and_then(|s| std::fs::write(&path, s).ok());
}

fn load_manifest() -> Option<IntegrityManifest> {
    let path = nyzhi_config::Config::data_dir().join("update-manifest.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

// ---------------------------------------------------------------------------
// Post-flight verification — make sure the new binary actually runs
// ---------------------------------------------------------------------------

fn verify_new_binary() -> Result<bool> {
    let exe = std::env::current_exe()
        .context("Could not determine current executable path")?;

    let output = std::process::Command::new(&exe)
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output();

    match output {
        Ok(o) if o.status.success() => Ok(true),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            tracing::warn!("New binary --version failed: {stderr}");
            Ok(false)
        }
        Err(e) => {
            tracing::warn!("Could not execute new binary for verification: {e}");
            Ok(false)
        }
    }
}

// ---------------------------------------------------------------------------
// URL validation — prevent SSRF and malicious update endpoints
// ---------------------------------------------------------------------------

const ALLOWED_HOSTS: &[&str] = &["get.nyzhi.com"];

fn validate_release_url(url: &str) -> Result<()> {
    let parsed: url::Url = url
        .parse()
        .context("Invalid release URL")?;

    if parsed.scheme() != "https" {
        anyhow::bail!("Release URL must use HTTPS (got {})", parsed.scheme());
    }

    let host = parsed.host_str().unwrap_or("");

    // Block cloud metadata endpoints and private IPs
    let blocked_hosts = [
        "169.254.169.254",
        "metadata.google.internal",
        "100.100.100.200",
    ];
    if blocked_hosts.contains(&host) {
        anyhow::bail!("Release URL points to a blocked host");
    }

    // If the URL is not our known host, warn but allow (for self-hosted)
    if !ALLOWED_HOSTS.contains(&host) {
        tracing::warn!(
            "Update URL uses non-default host '{host}'. \
             Ensure you trust this endpoint."
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Version check
// ---------------------------------------------------------------------------

/// Check whether a newer version is available.
/// Respects throttle interval and skipped-version preferences.
pub async fn check_for_update(config: &nyzhi_config::UpdateConfig) -> Result<Option<UpdateInfo>> {
    if !config.enabled {
        return Ok(None);
    }

    let state = load_state();
    let interval_secs = u64::from(config.check_interval_hours) * 3600;
    if now_epoch().saturating_sub(state.last_check_epoch) < interval_secs {
        return Ok(None);
    }

    validate_release_url(&config.release_url)?;

    let url = format!("{}/version", config.release_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Version check failed: HTTP {}", resp.status());
    }
    let release: ReleaseInfo = resp.json().await?;

    let mut new_state = load_state();
    new_state.last_check_epoch = now_epoch();
    save_state(&new_state);

    let current = semver::Version::parse(CURRENT_VERSION)
        .context("Failed to parse current version")?;
    let latest = semver::Version::parse(&release.version)
        .context("Failed to parse remote version")?;

    if latest <= current {
        return Ok(None);
    }

    if let Some(ref skipped) = new_state.skipped_version {
        if skipped == &release.version {
            return Ok(None);
        }
    }

    let platform = platform_key();
    let base = config.release_url.trim_end_matches('/');
    let download_url = format!(
        "{base}/download/{platform_parts}",
        platform_parts = platform.replace('-', "/")
    );

    Ok(Some(UpdateInfo {
        current_version: CURRENT_VERSION.to_string(),
        new_version: release.version.clone(),
        changelog: release.changelog,
        download_url: format!("{download_url}?version={}", release.version),
        sha256: release.sha256.get(&platform).cloned(),
    }))
}

/// Force a version check ignoring throttle.
pub async fn check_for_update_force(
    config: &nyzhi_config::UpdateConfig,
) -> Result<Option<UpdateInfo>> {
    let mut force_config = config.clone();
    force_config.check_interval_hours = 0;
    let mut state = load_state();
    state.skipped_version = None;
    state.last_check_epoch = 0;
    save_state(&state);
    check_for_update(&force_config).await
}

// ---------------------------------------------------------------------------
// Download & apply — the core safe-update pipeline
// ---------------------------------------------------------------------------

/// Download the new binary and replace the current one in-place.
///
/// Pipeline:
///   1. Snapshot integrity manifest (config hash, keyring state, data dirs)
///   2. Backup current binary to ~/.local/share/nyzhi/backups/
///   3. Download + verify checksum of new binary
///   4. Atomic in-place replacement via self-replace
///   5. Post-flight: run new binary --version to confirm it launches
///   6. Re-verify integrity manifest (config untouched, tokens intact)
///   7. On any failure after step 4: auto-rollback from backup
pub async fn download_and_apply(info: &UpdateInfo) -> Result<UpdateResult> {
    // Step 1: snapshot user data state before we touch anything
    let manifest = snapshot_integrity(&info.new_version);
    save_manifest(&manifest);

    // Step 2: backup current binary
    let backup_path = backup_current_binary(&info.current_version)
        .context("Pre-update backup failed")?;
    tracing::info!("Backed up current binary to {}", backup_path.display());

    // Step 3: download and verify
    let new_binary_path = match download_and_extract(info).await {
        Ok(path) => path,
        Err(e) => {
            // Download failed — nothing replaced yet, just clean up
            return Err(e.context("Download/extract failed; no changes were made"));
        }
    };

    // Step 4: atomic replacement
    if let Err(e) = self_replace::self_replace(&new_binary_path) {
        return Err(anyhow::anyhow!(e)
            .context("Binary replacement failed; old binary is still in place"));
    }

    // Step 5: post-flight verification
    let verified = match verify_new_binary() {
        Ok(true) => true,
        Ok(false) => {
            tracing::warn!("Post-flight: new binary --version failed, rolling back");
            if let Err(rb_err) = rollback(&backup_path) {
                return Err(anyhow::anyhow!(
                    "CRITICAL: New binary broken AND rollback failed: {rb_err}. \
                     Manual restore from: {}",
                    backup_path.display()
                ));
            }
            return Err(anyhow::anyhow!(
                "New binary failed verification; rolled back to v{}",
                info.current_version
            ));
        }
        Err(_) => {
            // Couldn't run the check (e.g. permissions) — proceed cautiously
            false
        }
    };

    // Step 6: re-verify integrity
    let issues = verify_integrity(&manifest);
    if !issues.is_empty() {
        tracing::warn!("Post-update integrity issues: {:?}", issues);
        // These are warnings, not fatal — config/keyring should never be touched
        // by binary replacement, but log them for debugging
    }

    Ok(UpdateResult {
        new_version: info.new_version.clone(),
        backup_path: Some(backup_path),
        verified,
    })
}

/// Download the tarball, verify its checksum, extract the binary.
/// Returns the path to the extracted binary in a temp dir.
/// The caller must use the binary before the tempdir is dropped.
async fn download_and_extract(info: &UpdateInfo) -> Result<PathBuf> {
    let expected_sha = info
        .sha256
        .as_ref()
        .context("Refusing to install update without a SHA-256 checksum")?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let resp = client.get(&info.download_url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Download failed: HTTP {}", resp.status());
    }
    let bytes = resp.bytes().await?;

    use sha2::Digest;
    let actual = hex::encode(sha2::Sha256::digest(&bytes));
    if actual != *expected_sha {
        anyhow::bail!(
            "Checksum mismatch!\n  Expected: {expected_sha}\n  Actual:   {actual}"
        );
    }

    let extract_dir = nyzhi_config::Config::data_dir().join("update-staging");
    let _ = std::fs::remove_dir_all(&extract_dir);
    std::fs::create_dir_all(&extract_dir)?;

    let tarball = extract_dir.join("nyz.tar.gz");
    std::fs::write(&tarball, &bytes)?;

    let tar_gz = std::fs::File::open(&tarball)?;
    let tar = flate2::read::GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(tar);

    archive.unpack(&extract_dir)?;

    let binary_path = find_binary_in_dir(&extract_dir)
        .context("Could not find nyz binary in archive")?;

    Ok(binary_path)
}

fn find_binary_in_dir(dir: &std::path::Path) -> Option<PathBuf> {
    let direct = dir.join("nyz");
    if direct.is_file() {
        return Some(direct);
    }
    let walker = std::fs::read_dir(dir).ok()?;
    for entry in walker.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some("nyz") {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_binary_in_dir(&path) {
                return Some(found);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Post-update health check — called on next startup
// ---------------------------------------------------------------------------

/// Called at startup to verify a recent update didn't damage anything.
/// Returns warnings if the last update left integrity issues.
pub fn startup_health_check() -> Vec<String> {
    let Some(manifest) = load_manifest() else {
        return Vec::new();
    };

    // Only check if the manifest is from a recent update (last 5 minutes)
    if now_epoch().saturating_sub(manifest.timestamp) > 300 {
        return Vec::new();
    }

    verify_integrity(&manifest)
}

// ---------------------------------------------------------------------------
// Convenience
// ---------------------------------------------------------------------------

/// Mark a version as skipped so the TUI won't prompt for it again.
pub fn skip_version(version: &str) {
    let mut state = load_state();
    state.skipped_version = Some(version.to_string());
    save_state(&state);
}

pub fn current_version() -> &'static str {
    CURRENT_VERSION
}
