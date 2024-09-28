use anyhow::Context;
use libsystemd::daemon::{notify, NotifyState};
use nix::time::{clock_gettime, ClockId};

pub async fn sd_notify(notify_type: SdNotifyType) -> anyhow::Result<()> {
    match notify_type {
        SdNotifyType::Ready => {
            tokio::task::spawn_blocking(|| notify(false, &[NotifyState::Ready]).unwrap_or(false))
                .await
                .context("Spawning as blocking a READY=1 call to sd_notify()")?;
            Ok(())
        }
        SdNotifyType::Reload => {
            let now = notify_state_monotonic_usec()
                .context("Calling clock_gettime with CLOCK_MONOTONIC")?;

            {
                let now = now.clone();
                tokio::task::spawn_blocking(move || {
                    notify(false, &[NotifyState::Reloading, now]).unwrap_or(false)
                })
            }
            .await
            .context(format!(
                "Spawning as blocking [RELOADING=1;MONOTONIC_USEC={}] call to sd_notify()",
                now
            ))?;

            Ok(())
        }
    }
}

pub enum SdNotifyType {
    Ready,
    Reload,
}

pub fn notify_state_monotonic_usec() -> anyhow::Result<NotifyState> {
    let timespec = clock_gettime(ClockId::CLOCK_MONOTONIC)?;
    Ok(NotifyState::Other(format!(
        "MONOTONIC_USEC={}",
        // Seconds to micros        // Nanos to micros
        timespec.tv_sec() * 1000000 + timespec.tv_nsec() / 1000
    )))
}
