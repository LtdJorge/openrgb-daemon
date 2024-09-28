mod cli;
mod config;
mod error;
mod notify;
mod signals;

use crate::config::LogLevel;
use crate::{
    cli::Arguments,
    config::{Configuration, Server},
    error::Error,
    notify::{sd_notify, SdNotifyType},
    signals::{handle_exit, handle_reload},
};
use anyhow::Context;
use clap::Parser;
use std::{env, fs::File, io::Read, path::PathBuf, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, ChildStderr, ChildStdout, Command},
};
use tokio_stream::{wrappers::LinesStream, StreamExt};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing::metadata::LevelFilter;
use tracing::{error, info, instrument, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

fn main() -> anyhow::Result<()> {
    let Arguments {
        config_file,
        passthrough_options,
        binary,
        log_level,
    } = Arguments::parse();

    // Cache whether we are running under systemd and as notify/notify-reload
    let is_notify: bool = libsystemd::daemon::booted() && env::var("NOTIFY_SOCKET").ok().is_some();

    let config_path = config_file.and_then(|path| path.canonicalize().ok());

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Building Tokio runtime")?;

    loop {
        let mut config: Configuration =
            config_path
                .clone()
                .map_or(anyhow::Ok(Configuration::default()), |path| {
                    let mut config_file = File::options().read(true).write(false).open(path)?;
                    let mut config_content = String::new();
                    config_file
                        .read_to_string(&mut config_content)
                        .context("Reading valid UTF-8 from TOML file to string")?;
                    let configuration = toml::from_str::<Configuration>(&config_content)
                        .context("Parsing TOML from configuration file")?;
                    Ok(configuration)
                })?;

        if let Some(ref passthrough) = passthrough_options {
            // Server port
            if let Some(server_port) = passthrough.server_port {
                config.server = match config.server {
                    None => Some(Server {
                        port: Some(server_port),
                        ..Default::default()
                    }),
                    Some(mut server) => {
                        server.port = Some(server_port);
                        Some(server)
                    }
                }
            }
            // Device
            if let Some(device) = &passthrough.device {
                config.devices.get_or_insert(vec![]).push(device.clone());
            }
            // Profile
            if let Some(profile) = &passthrough.profile {
                config.server = match config.server {
                    None => Some(Server {
                        profile: Some(profile.clone()),
                        ..Default::default()
                    }),
                    Some(mut server) => {
                        server.profile = Some(profile.clone());
                        Some(server)
                    }
                }
            }
            // Config
            if let Some(new_config) = &passthrough.config {
                let server = config.server.get_or_insert(Default::default());
                server.config = Some(new_config.clone());
            }
        }

        if let Some(ref level) = log_level {
            if config.log_level.is_none() {
                config.log_level = Some(level.clone())
            }
        }

        let level: Level = match config.log_level {
            None => Level::INFO,
            Some(ref log_level) => match log_level {
                LogLevel::Error => Level::ERROR,
                LogLevel::Warning => Level::WARN,
                LogLevel::Notice => Level::INFO,
                LogLevel::Informational => Level::DEBUG,
                LogLevel::Debug => Level::TRACE,
            },
        };

        let _subscriber_guard = match is_notify {
            true => tracing_subscriber::registry()
                .with(tracing_journald::Layer::new()?.with_filter(LevelFilter::from_level(level)))
                .set_default(),
            false => {
                // This guard gets dropped on reload and a new tracing_subscriber is built, taking
                // new configuration into account
                tracing_subscriber::fmt()
                    .with_max_level(level)
                    .finish()
                    .set_default()
            }
        };

        let reload = rt
            .block_on(run_openrgb(is_notify, config.clone(), binary.clone()))
            .context("Blocking on run_openrgb()")?;

        if reload {
            continue;
        }

        break;
    }

    Ok(())
}

#[instrument(skip(configuration))]
async fn run_openrgb(
    is_notify: bool,
    configuration: Configuration,
    binary_path: PathBuf,
) -> anyhow::Result<bool> {
    let mut command = Command::new(binary_path);
    let mut command_ref = &mut command;

    if let Some(server) = configuration.server {
        if let Some(port) = server.port {
            command_ref = command_ref.arg("--server-port").arg(port.to_string());
        }
        if let Some(config) = server.config {
            command_ref = command_ref.arg("--config").arg(config);
        }
        if let Some(profile) = server.profile {
            command_ref = command_ref.arg("--profile").arg(profile);
        }
    }

    for device in configuration.devices.unwrap_or_default() {
        if let Some(device_name) = device.device {
            command_ref = command_ref.arg("--device").arg(device_name);
        }
        if let Some(zone) = device.zone {
            command_ref = command_ref.arg("--zone").arg(zone.to_string());
        }
        if let Some(color) = device.color {
            command_ref = command_ref.arg("--color").arg(color.join(","));
        }
        if let Some(mode) = device.mode {
            command_ref = command_ref.arg("--mode").arg(mode);
        }
        if let Some(brightness) = device.brightness {
            command_ref = command_ref.arg("--brightness").arg(brightness.to_string());
        }
        if let Some(size) = device.size {
            command_ref = command_ref.arg("--size").arg(size.to_string());
        }
    }

    let mut openrgb_process = command_ref
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context(format!(
            "Spawning the fully built command: {:?}",
            command_ref
        ))?;

    let stdout = openrgb_process.stdout.take().ok_or(anyhow::anyhow!(
        "Couldn't capture stdout of openrgb process"
    ))?;

    let stderr = openrgb_process.stderr.take().ok_or(anyhow::anyhow!(
        "Couldn't capture stderr of openrgb process"
    ))?;

    if is_notify {
        sd_notify(SdNotifyType::Ready).await?;
    }

    // let (send, recv) = tokio::sync::mpsc::channel::<()>(1);

    let tracker = TaskTracker::new();
    let token = CancellationToken::new();

    let stdout_lines = LinesStream::new(BufReader::new(stdout).lines());
    let stderr_lines = LinesStream::new(BufReader::new(stderr).lines());

    tracker.spawn(child_process_loop(
        stdout_lines,
        stderr_lines,
        token.clone(),
        openrgb_process,
    ));

    tracker.spawn(handle_exit(token.clone()));

    let reload = tracker.spawn(handle_reload(token.clone()));

    tracker.close();
    tracker.wait().await;

    let reload = reload.await;

    let should_reload = match reload? {
        Ok(_) => false,
        Err(e) => matches!(e.downcast_ref::<Error>(), Some(Error::Reload)),
    };

    if should_reload {
        if is_notify {
            sd_notify(SdNotifyType::Reload).await?;
        }
        info!("Reloading")
    };

    Ok(should_reload)
}

#[instrument(skip(child))]
async fn kill_openrgb_child(child: &mut Child) -> anyhow::Result<()> {
    info!("Killing openrgb process");
    child.kill().await?;
    Ok(())
}

#[instrument(skip(stdout_lines, stderr_lines, token, openrgb_process))]
async fn child_process_loop(
    mut stdout_lines: LinesStream<BufReader<ChildStdout>>,
    mut stderr_lines: LinesStream<BufReader<ChildStderr>>,
    token: CancellationToken,
    mut openrgb_process: Child,
) -> anyhow::Result<()> {
    while !token.is_cancelled() {
        tokio::select! {
            out = stdout_lines.next() => {
                if let Some(Ok(line)) = out {
                    info!(line)
                }
            },
            err = stderr_lines.next() => {
                if let Some(Ok(line)) = err {
                    error!(line)
                }
            },
            _ = token.cancelled() => {
                return kill_openrgb_child(&mut openrgb_process).await
            }
            _ = openrgb_process.wait() => {
                info!("Openrgb process exited, exiting");
                token.cancel();
            }
        }
    }

    Ok(())
}
