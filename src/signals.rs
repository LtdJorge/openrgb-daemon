use crate::error::Error;
use anyhow::Context;
use tokio::signal::unix::{signal, SignalKind};
use tokio_util::sync::CancellationToken;
use tracing::{info, instrument};

#[instrument(skip(token))]
pub(crate) async fn handle_reload(token: CancellationToken) -> anyhow::Result<()> {
    let sighup = async {
        signal(SignalKind::hangup())
            .context("Setting up signal handler for SIGHUP")?
            .recv()
            .await;
        info!("Received SIGHUP, reloading");
        anyhow::Ok(())
    };

    tokio::select! {
        _ = sighup => {},
        _ = token.cancelled() => {
            return Ok(())
        }
    }
    if !token.is_cancelled() {
        token.cancel()
    };

    Err(Error::Reload.into())
}

#[instrument(skip(token))]
pub(crate) async fn handle_exit(token: CancellationToken) -> anyhow::Result<()> {
    let interrupt = async {
        signal(SignalKind::interrupt())
            .context("Setting up signal handler for SIGHUP")?
            .recv()
            .await;
        info!("Received SIGINT, exiting");
        anyhow::Ok(())
    };

    let terminate = async {
        signal(SignalKind::terminate())
            .context("Setting up signal handler for SIGHUP")?
            .recv()
            .await;
        info!("Received SIGTERM, exiting");
        anyhow::Ok(())
    };

    tokio::select! {
        _ = interrupt => {},
        _ = terminate => {},
        _ = token.cancelled() => {}
    }

    if !token.is_cancelled() {
        token.cancel()
    };

    Ok(())
}
