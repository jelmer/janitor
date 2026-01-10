use nix::unistd::{close, dup, dup2};
use std::fs::File;
use std::io::{self, Write};
use std::os::fd::{BorrowedFd, FromRawFd, OwnedFd};
use std::os::unix::io::{AsRawFd, RawFd};
use std::process::{Command, Stdio};

pub struct CopyOutput {
    old_stdout: OwnedFd,
    old_stderr: OwnedFd,
    tee: bool,
    process: Option<std::process::Child>,
    newfd: Option<File>,
}

impl CopyOutput {
    pub fn new(output_log: &std::path::Path, tee: bool) -> io::Result<Self> {
        let old_stdout = dup(unsafe { BorrowedFd::borrow_raw(nix::libc::STDOUT_FILENO) })
            .expect("Failed to duplicate stdout");
        let old_stderr = dup(unsafe { BorrowedFd::borrow_raw(nix::libc::STDERR_FILENO) })
            .expect("Failed to duplicate stderr");

        let mut process = None;
        let newfd: Option<File>;

        if tee {
            let p = Command::new("tee")
                .arg(output_log)
                .stdin(Stdio::piped())
                .spawn()?;
            let mut stdout_fd = unsafe { OwnedFd::from_raw_fd(nix::libc::STDOUT_FILENO) };
            let mut stderr_fd = unsafe { OwnedFd::from_raw_fd(nix::libc::STDERR_FILENO) };
            dup2(p.stdin.as_ref().unwrap(), &mut stdout_fd)
                .expect("Failed to redirect stdout to tee");
            dup2(p.stdin.as_ref().unwrap(), &mut stderr_fd)
                .expect("Failed to redirect stderr to tee");
            std::mem::forget(stdout_fd);
            std::mem::forget(stderr_fd);
            process = Some(p);
            newfd = None;
        } else {
            let file = File::create(output_log)?;
            let mut stdout_fd = unsafe { OwnedFd::from_raw_fd(nix::libc::STDOUT_FILENO) };
            let mut stderr_fd = unsafe { OwnedFd::from_raw_fd(nix::libc::STDERR_FILENO) };
            dup2(&file, &mut stdout_fd).expect("Failed to redirect stdout to file");
            dup2(&file, &mut stderr_fd).expect("Failed to redirect stderr to file");
            std::mem::forget(stdout_fd);
            std::mem::forget(stderr_fd);
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
        let mut stdout_fd = unsafe { OwnedFd::from_raw_fd(nix::libc::STDOUT_FILENO) };
        let mut stderr_fd = unsafe { OwnedFd::from_raw_fd(nix::libc::STDERR_FILENO) };
        dup2(&self.old_stdout, &mut stdout_fd).expect("Failed to restore stdout");
        dup2(&self.old_stderr, &mut stderr_fd).expect("Failed to restore stderr");
        std::mem::forget(stdout_fd); // Don't close stdout/stderr
        std::mem::forget(stderr_fd);

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
