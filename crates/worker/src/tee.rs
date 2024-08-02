use nix::unistd::{close, dup, dup2};
use std::fs::File;
use std::io::{self, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::process::{Command, Stdio};

struct CopyOutput {
    old_stdout: RawFd,
    old_stderr: RawFd,
    tee: bool,
    process: Option<std::process::Child>,
    newfd: Option<File>,
}

impl CopyOutput {
    fn new(output_log: &str, tee: bool) -> io::Result<Self> {
        let old_stdout = dup(nix::libc::STDOUT_FILENO).expect("Failed to duplicate stdout");
        let old_stderr = dup(nix::libc::STDERR_FILENO).expect("Failed to duplicate stderr");

        let mut process = None;
        let newfd: Option<File>;

        if tee {
            let p = Command::new("tee")
                .arg(output_log)
                .stdin(Stdio::piped())
                .spawn()?;
            dup2(
                p.stdin.as_ref().unwrap().as_raw_fd(),
                nix::libc::STDOUT_FILENO,
            )
            .expect("Failed to redirect stdout to tee");
            dup2(
                p.stdin.as_ref().unwrap().as_raw_fd(),
                nix::libc::STDERR_FILENO,
            )
            .expect("Failed to redirect stderr to tee");
            process = Some(p);
            newfd = None;
        } else {
            let file = File::create(output_log)?;
            dup2(file.as_raw_fd(), nix::libc::STDOUT_FILENO)
                .expect("Failed to redirect stdout to file");
            dup2(file.as_raw_fd(), nix::libc::STDERR_FILENO)
                .expect("Failed to redirect stderr to file");
            newfd = Some(file);
        }

        Ok(Self {
            old_stdout,
            old_stderr,
            tee,
            process,
            newfd,
        })
    }
}

impl Drop for CopyOutput {
    fn drop(&mut self) {
        // Restore original stdout and stderr
        dup2(self.old_stdout, nix::libc::STDOUT_FILENO).expect("Failed to restore stdout");
        dup2(self.old_stderr, nix::libc::STDERR_FILENO).expect("Failed to restore stderr");
        close(self.old_stdout).expect("Failed to close old stdout");
        close(self.old_stderr).expect("Failed to close old stderr");

        // Ensure process or file is cleaned up
        if self.tee {
            if let Some(ref mut process) = self.process {
                process.wait().expect("Failed to wait on tee process");
            }
        } else if let Some(ref mut file) = self.newfd {
            file.flush().expect("Failed to flush output file");
        }
    }
}
