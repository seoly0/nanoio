use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "nanoio",
    about = "High-performance S3-compatible local object storage server in Rust",
    version
)]
pub struct Config {
    /// Bind address
    #[arg(short = 'H', long, default_value = "0.0.0.0", env = "NANOIO_HOST")]
    pub host: String,

    /// Bind port (defaults to 9000 for S3 API compatibility)
    #[arg(short, long, default_value_t = 9000, env = "NANOIO_PORT")]
    pub port: u16,

    /// Web console port (defaults to 9001 for Console compatibility)
    #[arg(long, default_value_t = 9001, env = "NANOIO_CONSOLE_PORT")]
    pub console_port: u16,

    /// Disable web console
    #[arg(long, env = "NANOIO_NO_CONSOLE")]
    pub no_console: bool,

    /// Directory path where buckets and objects are stored
    #[arg(short, long, default_value = "./data", env = "NANOIO_DATA_DIR")]
    pub data_dir: PathBuf,

    /// AWS Access Key ID (default: nanoioadmin)
    #[arg(long, default_value = "nanoioadmin", env = "NANOIO_ACCESS_KEY_ID")]
    pub access_key_id: String,

    /// AWS Secret Access Key (default: nanoioadmin)
    #[arg(long, default_value = "nanoioadmin", env = "NANOIO_SECRET_ACCESS_KEY")]
    pub secret_access_key: String,

    /// Authentication mode: 'strict' or 'permissive' (default: strict)
    #[arg(long, default_value = "strict", env = "NANOIO_AUTH_MODE")]
    pub auth_mode: String,

    /// Optional IAM users in comma-separated format: 'key:secret[:policy],key2:secret2[:policy]'
    /// policy can be 'admin', 'readwrite', or 'readonly' (default: admin)
    #[arg(long, env = "NANOIO_IAM_USERS")]
    pub iam_users: Option<String>,

    /// Optional default bucket(s) to create automatically on startup (comma-separated, e.g. 'my-bucket')
    #[arg(long, env = "NANOIO_DEFAULT_BUCKET")]
    pub default_bucket: Option<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info", env = "RUST_LOG")]
    pub log_level: String,
}

impl Config {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
