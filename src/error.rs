use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("Systemd sent a reload signal")]
    Reload,
}
