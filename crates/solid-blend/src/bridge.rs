use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use solid_rs::SolidError;

const DEFAULT_TIMEOUT_SECS: u64 = 180;
const POLL_INTERVAL_MS: u64 = 100;

pub(crate) struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    pub(crate) fn new(prefix: &str) -> Result<Self, SolidError> {
        let mut path = std::env::temp_dir();
        let unique = format!(
            "{}-{}-{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| SolidError::other(format!("clock error: {e}")))?
                .as_nanos()
        );
        path.push(unique);
        fs::create_dir_all(&path).map_err(SolidError::Io)?;
        Ok(Self { path })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(crate) fn blender_bin() -> OsString {
    std::env::var_os("BLENDER_BIN").unwrap_or_else(|| OsString::from("blender"))
}

pub(crate) fn run_blender(args: &[OsString]) -> Result<(), SolidError> {
    let timeout = blender_timeout();
    let mut cmd = Command::new(blender_bin());
    cmd.args(args).stdout(Stdio::null()).stderr(Stdio::null());

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            SolidError::unsupported(
                "Blender executable not found; set BLENDER_BIN or install Blender",
            )
        } else {
            SolidError::Io(e)
        }
    })?;

    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(SolidError::Io)? {
            if status.success() {
                return Ok(());
            }
            return Err(SolidError::format(
                "blend",
                format!("Blender conversion failed with status {status}"),
            ));
        }

        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(SolidError::unsupported(format!(
                "Blender conversion timed out after {} seconds",
                timeout.as_secs()
            )));
        }

        std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
    }
}

fn blender_timeout() -> Duration {
    let secs = std::env::var("BLENDER_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_TIMEOUT_SECS);
    Duration::from_secs(secs)
}
