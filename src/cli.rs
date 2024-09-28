use crate::config::{Device, LogLevel};
use clap::{Args, Parser};
use std::path::PathBuf;

/// TODO: complete this
///
/// The path to a TOML configuration file for this daemon. If not present, no configuration will be assumed and the server will be started as: openrgb --server [..other_options] All other options found will be parsed and checked for correctness, but will be passed through to the openrgb --server call. For most use cases, none should be specified. The best way to configure OpenRGB when running as a daemon is through its config file and profile, by executing another OpenRGB instance as client.
#[derive(Debug, Parser)]
#[command(version)]
pub struct Arguments {
    /// The path to a TOML configuration file for this daemon.
    /// If not present, no configuration will be assumed and the server will be started as:
    ///     `openrgb --server [..other_options]`
    /// All other options found will be parsed and checked for correctness,
    /// but will be passed through to the `openrgb --server` call.
    /// For most use cases, none should be specified. The best way to configure OpenRGB when running
    /// as a daemon is through its config file and profile, by executing another OpenRGB instance as client.
    #[arg(long = "path", value_name = "config_file", verbatim_doc_comment)]
    pub config_file: Option<PathBuf>,

    #[arg(long, value_name = "log_level")]
    pub log_level: Option<LogLevel>,

    #[command(flatten)]
    pub passthrough_options: Option<Passthrough>,

    /// This is the path to the openrgb executable
    #[arg(last = true)]
    pub binary: PathBuf,
}

#[derive(Args, Debug)]
pub struct Passthrough {
    /// Sets the SDK's server port. Default: 6742 (1024-65535)
    #[arg(long, value_name = "port")]
    pub server_port: Option<u16>,

    #[clap(flatten)]
    pub device: Option<Device>,

    /// Load the profile from filename/filename.orp
    #[arg(short, long, value_name = "filename[.orp]")]
    pub profile: Option<String>,

    /// Use a custom path instead of the global configuration directory.
    /// NOTE: this setting is best configured in the TOML file passed to `--path`
    #[arg(long, value_name = "path", verbatim_doc_comment)]
    pub config: Option<PathBuf>,
}
