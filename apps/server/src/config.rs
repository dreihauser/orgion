//! `orgion.toml` configuration, with environment variable overrides
//! (`ORGION_SERVER_PORT`, etc.) — see docs (`§28 Configuration` in the
//! original project brief).

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub index: IndexConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            host: default_host(),
            port: default_port(),
        }
    }
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    3030
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceConfig {
    pub root: PathBuf,
    /// Opaque key namespacing this workspace's rows in the index DB.
    /// Defaults to "default" for the common single-workspace, single-user
    /// self-hosted setup.
    #[serde(default = "default_workspace_key")]
    pub key: String,
}

fn default_workspace_key() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_database_url")]
    pub url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            url: default_database_url(),
        }
    }
}

fn default_database_url() -> String {
    "sqlite://./orgion-index.db".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    /// Requires a session cookie (from `POST /api/auth/login`) on every
    /// API route except `/api/auth/*`. `orgion init` creates the admin
    /// account this logs into. Set to `false` for a quick local trial
    /// with no login screen (docs/mvp.md).
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig { enabled: true }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndexConfig {
    #[serde(default = "default_true")]
    pub watch: bool,
}

impl Default for IndexConfig {
    fn default() -> Self {
        IndexConfig { watch: true }
    }
}

fn default_true() -> bool {
    true
}

impl Config {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Config> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?;
        let mut config: Config = toml::from_str(&text)
            .map_err(|e| anyhow::anyhow!("parsing {}: {e}", path.display()))?;

        if let Ok(port) = std::env::var("ORGION_SERVER_PORT") {
            config.server.port = port.parse()?;
        }
        if let Ok(host) = std::env::var("ORGION_SERVER_HOST") {
            config.server.host = host;
        }
        if let Ok(url) = std::env::var("ORGION_DATABASE_URL") {
            config.database.url = url;
        }
        if let Ok(root) = std::env::var("ORGION_WORKSPACE_ROOT") {
            config.workspace.root = PathBuf::from(root);
        }
        Ok(config)
    }

    pub fn write_default(path: &std::path::Path, workspace_root: &std::path::Path) -> anyhow::Result<()> {
        let contents = format!(
            "[server]\nhost = \"{}\"\nport = {}\n\n[workspace]\nroot = \"{}\"\nkey = \"default\"\n\n[database]\nurl = \"{}\"\n\n[auth]\nenabled = true\n\n[index]\nwatch = true\n",
            default_host(),
            default_port(),
            workspace_root.display(),
            default_database_url(),
        );
        std::fs::write(path, contents)?;
        Ok(())
    }
}
