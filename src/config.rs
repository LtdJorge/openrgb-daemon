use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Configuration {
    pub log_level: Option<LogLevel>,
    pub server: Option<Server>,
    pub devices: Option<Vec<Device>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warning,
    #[default]
    Notice,
    Informational,
    Debug,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Server {
    pub config: Option<PathBuf>,
    pub profile: Option<String>,
    pub port: Option<u16>,
}
#[derive(Args, Clone, Debug, Deserialize, Serialize)]
pub struct Device {
    /// Selects device to apply colors and/or effect to, or applies to all devices if omitted
    ///   Basic string search is implemented 3 characters or more
    ///   Can be specified multiple times with different modes and colors
    #[arg(short, long, value_name = "0-9 | \"name\"", verbatim_doc_comment)]
    pub device: Option<String>,

    /// Selects zone to apply colors and/or sizes to, or applies to all zones in device if omitted
    ///   Must be specified after specifying a device
    #[arg(
        short,
        long,
        requires = "device",
        value_name = "0-9",
        verbatim_doc_comment
    )]
    pub zone: Option<u8>,

    /// Sets colors on each device directly if no effect is specified, and sets the effect color if an effect is specified
    ///   If there are more LEDs than colors given, the last color will be applied to the remaining LEDs
    #[arg(
        short,
        long,
        value_delimiter = ',',
        value_name = "random | FFFFF,00AAFF ...",
        verbatim_doc_comment
    )]
    pub color: Option<Vec<String>>,

    /// Sets the mode to be applied, check --list-devices to see which modes are supported on your device
    #[arg(short, long, value_name = "breathing | static | ...")]
    pub mode: Option<String>,

    /// Sets the brightness as a percentage if the mode supports brightness
    #[arg(short, long, value_name = "0-100")]
    pub brightness: Option<u8>,

    /// Sets the new size of the specified device zone.
    ///    Must be specified after specifying a zone.
    ///    If the specified size is out of range, or the zone does not offer resizing capability, the size will not be changed
    #[arg(short, long, value_name = "0-N", verbatim_doc_comment)]
    pub size: Option<u64>,
}
