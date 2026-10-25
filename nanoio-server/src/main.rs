use nanoio::{
    console::create_console_router, create_router, domain::StorageError,
    storage::{fs::FsStorageEngine, StorageEngine}, Config,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::parse_args();

    // 로깅 초기화
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting nanoio...");
    info!("Data directory: {:?}", config.data_dir);

    // 스토리지 엔진 초기화
    let engine = Arc::new(FsStorageEngine::new(&config.data_dir).await?);

    // 기본 버킷 초기화 (NANOIO_DEFAULT_BUCKET)
    if let Some(default_buckets) = &config.default_bucket {
        for b_name in default_buckets.split(',') {
            let b_name = b_name.trim();
            if !b_name.is_empty() {
                match engine.create_bucket(b_name).await {
                    Ok(_) => info!("Initialized default bucket: '{}'", b_name),
                    Err(StorageError::BucketAlreadyExists(_)) => {
                        info!("Default bucket '{}' already exists", b_name);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to initialize default bucket '{}': {}", b_name, e);
                    }
                }
            }
        }
    }

    // 인증 모듈 초기화
    let auth_mode = nanoio::auth::AuthMode::parse(&config.auth_mode);
    info!("Auth mode: {:?}, Root AccessKey: '{}'", auth_mode, config.access_key_id);
    let mut auth_ctx = nanoio::auth::AuthContext::new(
        config.access_key_id.clone(),
        config.secret_access_key.clone(),
        auth_mode,
    );

    // 추가 IAM 사용자 등록 (NANOIO_IAM_USERS=key:secret:policy,...)
    if let Some(iam_str) = &config.iam_users {
        for user_entry in iam_str.split(',') {
            let parts: Vec<&str> = user_entry.trim().split(':').collect();
            if parts.len() >= 2 {
                let access_key = parts[0].trim().to_string();
                let secret_key = parts[1].trim().to_string();
                let policy = if parts.len() >= 3 {
                    nanoio::auth::IamPolicy::parse(parts[2].trim())
                } else {
                    nanoio::auth::IamPolicy::Admin
                };
                info!("Registered IAM User: '{}' with policy {:?}", access_key, policy);
                auth_ctx.add_user(nanoio::auth::IamUser {
                    access_key_id: access_key,
                    secret_access_key: secret_key,
                    policy,
                });
            }
        }
    }
    let auth = Arc::new(auth_ctx);

    // S3 REST API 라우터 초기화
    let s3_app = create_router(engine, auth);
    let s3_addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let s3_listener = tokio::net::TcpListener::bind(s3_addr).await?;

    info!("🚀 S3 API Server: http://{}", s3_addr);
    info!("📦 S3 Endpoint:   http://localhost:{}", config.port);

    let shutdown_rx = shutdown_signal();

    if !config.no_console {
        let console_app = create_console_router();
        let console_addr: SocketAddr = format!("{}:{}", config.host, config.console_port).parse()?;
        let console_listener = tokio::net::TcpListener::bind(console_addr).await?;

        info!("🖥️  Web Console:   http://localhost:{}", config.console_port);
        info!("💡 Tip: Open http://localhost:{} in your browser to manage your buckets and objects", config.console_port);

        let s3_server = axum::serve(s3_listener, s3_app);
        let console_server = axum::serve(console_listener, console_app);

        tokio::select! {
            res = s3_server => {
                if let Err(e) = res {
                    tracing::error!("S3 API server error: {e}");
                }
            }
            res = console_server => {
                if let Err(e) = res {
                    tracing::error!("Console server error: {e}");
                }
            }
            _ = shutdown_rx => {
                info!("Shutdown signal received, closing servers...");
            }
        }
    } else {
        info!("Web console is disabled.");
        axum::serve(s3_listener, s3_app)
            .with_graceful_shutdown(shutdown_rx)
            .await?;
    }

    info!("Server gracefully stopped.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to listen for SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, initiating graceful shutdown...");
        },
        _ = terminate => {
            info!("Received SIGTERM, initiating graceful shutdown...");
        },
    }
}
