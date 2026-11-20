use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::core::errors::AdtError;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl ProcessOutput {
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

/// Minimal abstraction for executing external read-only ADB processes,
/// enabling deterministic mock-based testing without physical hardware.
pub trait ProcessRunner: Send + Sync {
    fn run<'a>(
        &'a self,
        executable: &'a Path,
        args: &'a [String],
        command_label: &'a str,
        timeout: Duration,
    ) -> BoxFuture<'a, Result<ProcessOutput, AdtError>>;
}

#[derive(Debug, Default, Clone)]
pub struct TokioProcessRunner;

impl ProcessRunner for TokioProcessRunner {
    fn run<'a>(
        &'a self,
        executable: &'a Path,
        args: &'a [String],
        command_label: &'a str,
        timeout: Duration,
    ) -> BoxFuture<'a, Result<ProcessOutput, AdtError>> {
        Box::pin(async move {
            let mut cmd = Command::new(executable);
            cmd.args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true);

            // Isolate Windows-specific process flag to prevent console window flicker.
            #[cfg(windows)]
            {
                const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }

            let mut child = cmd.spawn().map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    AdtError::AdbNotFound
                } else {
                    AdtError::ProcessSpawnFailed {
                        message: err.to_string(),
                    }
                }
            })?;

            let mut stdout_pipe = child.stdout.take();
            let mut stderr_pipe = child.stderr.take();

            let stdout_task = tokio::spawn(async move {
                let mut buf = Vec::new();
                if let Some(ref mut pipe) = stdout_pipe {
                    let _ = pipe.read_to_end(&mut buf).await;
                }
                buf
            });

            let stderr_task = tokio::spawn(async move {
                let mut buf = Vec::new();
                if let Some(ref mut pipe) = stderr_pipe {
                    let _ = pipe.read_to_end(&mut buf).await;
                }
                buf
            });

            let wait_result = tokio::time::timeout(timeout, child.wait()).await;

            match wait_result {
                Ok(Ok(status)) => {
                    let stdout_bytes = stdout_task.await.unwrap_or_default();
                    let stderr_bytes = stderr_task.await.unwrap_or_default();

                    Ok(ProcessOutput {
                        exit_code: status.code(),
                        stdout: String::from_utf8_lossy(&stdout_bytes).into_owned(),
                        stderr: String::from_utf8_lossy(&stderr_bytes).into_owned(),
                    })
                }
                Ok(Err(err)) => {
                    stdout_task.abort();
                    stderr_task.abort();
                    Err(AdtError::ProcessSpawnFailed {
                        message: err.to_string(),
                    })
                }
                Err(_) => {
                    // Explicitly terminate the local ADB client process on timeout.
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    stdout_task.abort();
                    stderr_task.abort();

                    Err(AdtError::Timeout {
                        command: command_label.to_string(),
                        timeout_ms: u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
                    })
                }
            }
        })
    }
}

/// Deterministic queue-based mock process runner for unit and integration tests.
#[doc(hidden)]
pub mod mock {
    use std::collections::VecDeque;
    use std::path::Path;
    use std::sync::{Mutex, PoisonError};
    use std::time::Duration;

    use super::{BoxFuture, ProcessOutput, ProcessRunner};
    use crate::core::errors::AdtError;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RecordedInvocation {
        pub executable: String,
        pub args: Vec<String>,
        pub command_label: String,
    }

    pub struct MockProcessRunner {
        responses: Mutex<VecDeque<Result<ProcessOutput, AdtError>>>,
        invocations: Mutex<Vec<RecordedInvocation>>,
    }

    impl MockProcessRunner {
        pub fn new(responses: Vec<Result<ProcessOutput, AdtError>>) -> Self {
            Self {
                responses: Mutex::new(VecDeque::from(responses)),
                invocations: Mutex::new(Vec::new()),
            }
        }

        pub fn invocations(&self) -> Vec<RecordedInvocation> {
            self.invocations
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    impl ProcessRunner for MockProcessRunner {
        fn run<'a>(
            &'a self,
            executable: &'a Path,
            args: &'a [String],
            command_label: &'a str,
            _timeout: Duration,
        ) -> BoxFuture<'a, Result<ProcessOutput, AdtError>> {
            let invocation = RecordedInvocation {
                executable: executable.to_string_lossy().into_owned(),
                args: args.to_vec(),
                command_label: command_label.to_string(),
            };

            self.invocations
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(invocation);

            let next = self
                .responses
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .pop_front()
                .unwrap_or_else(|| {
                    Err(AdtError::ProcessFailed {
                        command: command_label.to_string(),
                        exit_code: None,
                        stderr: "MockProcessRunner queue exhausted".to_string(),
                    })
                });

            Box::pin(async move { next })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tokio_runner_maps_missing_binary_to_adb_not_found() {
        let runner = TokioProcessRunner;
        let missing = Path::new("adt_definitely_nonexistent_adb_binary_12345");
        let args = vec!["version".to_string()];

        let result = runner
            .run(missing, &args, "adb version", Duration::from_secs(2))
            .await;

        assert!(matches!(result, Err(AdtError::AdbNotFound)));
    }
}
