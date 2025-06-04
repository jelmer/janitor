use nix::unistd::{close, dup, dup2};
use std::fs::File;
use std::io::{self, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::process::{Command, Stdio};

/// Safe wrapper for capturing stdout/stderr to a file
///
/// This struct redirects stdout and stderr to either a file directly
/// or through the `tee` command for simultaneous console and file output.
/// File descriptors are properly managed to prevent leaks.
pub struct CopyOutput {
    old_stdout: Option<RawFd>,
    old_stderr: Option<RawFd>,
    tee: bool,
    process: Option<std::process::Child>,
    newfd: Option<File>,
}

impl CopyOutput {
    /// Create a new CopyOutput that redirects stdout/stderr to the specified file
    ///
    /// # Arguments
    /// * `output_log` - Path to the output log file
    /// * `tee` - If true, use `tee` command to show output on console and write to file
    ///
    /// # Safety
    /// This function manipulates file descriptors. If the process panics or exits
    /// unexpectedly, the original stdout/stderr may not be restored.
    pub fn new(output_log: &std::path::Path, tee: bool) -> io::Result<Self> {
        // Validate the output path
        if let Some(parent) = output_log.parent() {
            if !parent.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Parent directory does not exist: {}", parent.display()),
                ));
            }
        }

        // Safely duplicate file descriptors
        let old_stdout = dup(nix::libc::STDOUT_FILENO).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to duplicate stdout: {}", e),
            )
        })?;

        let old_stderr = dup(nix::libc::STDERR_FILENO).map_err(|e| {
            // Clean up stdout if stderr duplication fails
            let _ = close(old_stdout);
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to duplicate stderr: {}", e),
            )
        })?;

        let mut copy_output = Self {
            old_stdout: Some(old_stdout),
            old_stderr: Some(old_stderr),
            tee,
            process: None,
            newfd: None,
        };

        // Set up redirection
        if tee {
            copy_output.setup_tee_redirection(output_log)?;
        } else {
            copy_output.setup_file_redirection(output_log)?;
        }

        Ok(copy_output)
    }

    /// Set up redirection through the `tee` command
    fn setup_tee_redirection(&mut self, output_log: &std::path::Path) -> io::Result<()> {
        let process = Command::new("tee")
            .arg(output_log)
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to spawn tee process: {}", e),
                )
            })?;

        let stdin_fd = process
            .stdin
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Tee process has no stdin"))?
            .as_raw_fd();

        // Redirect stdout and stderr to tee's stdin
        dup2(stdin_fd, nix::libc::STDOUT_FILENO).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to redirect stdout to tee: {}", e),
            )
        })?;

        dup2(stdin_fd, nix::libc::STDERR_FILENO).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to redirect stderr to tee: {}", e),
            )
        })?;

        self.process = Some(process);
        Ok(())
    }

    /// Set up direct redirection to a file
    fn setup_file_redirection(&mut self, output_log: &std::path::Path) -> io::Result<()> {
        let file = File::create(output_log)?;
        let file_fd = file.as_raw_fd();

        // Redirect stdout and stderr to the file
        dup2(file_fd, nix::libc::STDOUT_FILENO).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to redirect stdout to file: {}", e),
            )
        })?;

        dup2(file_fd, nix::libc::STDERR_FILENO).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to redirect stderr to file: {}", e),
            )
        })?;

        self.newfd = Some(file);
        Ok(())
    }

    /// Manually restore the original stdout/stderr
    ///
    /// This is called automatically by Drop, but can be called manually
    /// if you need to restore output before the CopyOutput is dropped.
    pub fn restore(&mut self) -> io::Result<()> {
        if let (Some(old_stdout), Some(old_stderr)) =
            (self.old_stdout.take(), self.old_stderr.take())
        {
            // Restore original stdout and stderr
            dup2(old_stdout, nix::libc::STDOUT_FILENO).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to restore stdout: {}", e),
                )
            })?;

            dup2(old_stderr, nix::libc::STDERR_FILENO).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to restore stderr: {}", e),
                )
            })?;

            // Close the duplicated file descriptors
            close(old_stdout).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to close old stdout: {}", e),
                )
            })?;

            close(old_stderr).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to close old stderr: {}", e),
                )
            })?;
        }

        // Clean up tee process or file
        if self.tee {
            if let Some(ref mut process) = self.process.take() {
                process.wait().map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!("Failed to wait on tee process: {}", e),
                    )
                })?;
            }
        } else if let Some(ref mut file) = self.newfd.take() {
            file.flush().map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to flush output file: {}", e),
                )
            })?;
        }

        Ok(())
    }
}

impl Drop for CopyOutput {
    fn drop(&mut self) {
        // Attempt to restore file descriptors safely
        // In Drop we can't propagate errors, so we log them instead
        if let Err(e) = self.restore() {
            eprintln!(
                "Warning: Failed to restore file descriptors during drop: {}",
                e
            );
            // Continue with cleanup - don't panic in Drop
        }
    }
}
