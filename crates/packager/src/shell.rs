// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
    io::{BufRead, BufReader},
    process::{Command, ExitStatus, Output, Stdio},
    sync::{Arc, Mutex},
};

pub trait CommandExt {
    // The `piped` method sets the stdout and stderr to properly
    // show the command output in the Node.js wrapper.
    fn piped(&mut self) -> std::io::Result<ExitStatus>;
    fn output_ok(&mut self) -> std::io::Result<Output>;
}

impl CommandExt for Command {
    fn piped(&mut self) -> std::io::Result<ExitStatus> {
        self.stdout(os_pipe::dup_stdout()?);
        self.stderr(os_pipe::dup_stderr()?);
        tracing::debug!("Running Command `{self:?}`");
        self.status().map_err(Into::into)
    }

    fn output_ok(&mut self) -> std::io::Result<Output> {
        tracing::debug!("Running Command `{self:?}`");

        self.stdout(Stdio::piped());
        self.stderr(Stdio::piped());

        let mut child = self.spawn()?;

        let mut stdout = child.stdout.take().map(BufReader::new).unwrap();
        let stdout_lines = Arc::new(Mutex::new(Vec::new()));
        let stdout_lines_ = stdout_lines.clone();
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let mut lines = stdout_lines_.lock().unwrap();
            loop {
                buf.clear();
                if let Ok(0) = stdout.read_until(b'\n', &mut buf) {
                    break;
                }
                tracing::debug!(
                    shell = "stdout",
                    "{}",
                    String::from_utf8_lossy(&buf[..buf.len() - 1])
                );
                lines.extend(&buf);
            }
        });

        let mut stderr = child.stderr.take().map(BufReader::new).unwrap();
        let stderr_lines = Arc::new(Mutex::new(Vec::new()));
        let stderr_lines_ = stderr_lines.clone();
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let mut lines = stderr_lines_.lock().unwrap();
            loop {
                buf.clear();
                if let Ok(0) = stderr.read_until(b'\n', &mut buf) {
                    break;
                }
                tracing::error!(
                    shell = "stderr",
                    "{}",
                    String::from_utf8_lossy(&buf[..buf.len() - 1])
                );
                lines.extend(&buf);
            }
        });

        let status = child.wait()?;
        let output = Output {
            status,
            stdout: std::mem::take(&mut *stdout_lines.lock().unwrap()),
            stderr: std::mem::take(&mut *stderr_lines.lock().unwrap()),
        };

        if output.status.success() {
            Ok(output)
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}
