use std::{
    io::{BufRead, BufReader},
    process::{Command, ExitStatus, Output, Stdio},
    sync::{Arc, Mutex},
};

use log::debug;

pub trait CommandExt {
    // The `pipe` function sets the stdout and stderr to properly
    // show the command output in the Node.js wrapper.
    fn piped(&mut self) -> std::io::Result<ExitStatus>;
    fn output_ok(&mut self) -> crate::Result<Output>;
}

impl CommandExt for Command {
    fn piped(&mut self) -> std::io::Result<ExitStatus> {
        self.stdout(os_pipe::dup_stdout()?);
        self.stderr(os_pipe::dup_stderr()?);
        let program = self.get_program().to_string_lossy().into_owned();
        debug!(action = "Running"; "Command `{} {}`", program, self.get_args().map(|arg| arg.to_string_lossy()).fold(String::new(), |acc, arg| format!("{acc} {arg}")));

        self.status().map_err(Into::into)
    }

    fn output_ok(&mut self) -> crate::Result<Output> {
        let program = self.get_program().to_string_lossy().into_owned();
        debug!(action = "Running"; "Command `{} {}`", program, self.get_args().map(|arg| arg.to_string_lossy()).fold(String::new(), |acc, arg| format!("{} {}", acc, arg)));

        self.stdout(Stdio::piped());
        self.stderr(Stdio::piped());

        let mut child = self.spawn()?;

        let mut stdout = child.stdout.take().map(BufReader::new).unwrap();
        let stdout_lines = Arc::new(Mutex::new(Vec::new()));
        let stdout_lines_ = stdout_lines.clone();
        std::thread::spawn(move || {
            let mut buf = String::new();
            let mut lines = stdout_lines_.lock().unwrap();
            loop {
                buf.clear();
                match stdout.read_line(&mut buf) {
                    Ok(s) if s == 0 => break,
                    _ => (),
                }
                debug!(action = "stdout"; "{buf}");
                lines.extend(buf.as_bytes().to_vec());
                lines.push(b'\n');
            }
        });

        let mut stderr = child.stderr.take().map(BufReader::new).unwrap();
        let stderr_lines = Arc::new(Mutex::new(Vec::new()));
        let stderr_lines_ = stderr_lines.clone();
        std::thread::spawn(move || {
            let mut buf = String::new();
            let mut lines = stderr_lines_.lock().unwrap();
            loop {
                buf.clear();
                match stderr.read_line(&mut buf) {
                    Ok(s) if s == 0 => break,
                    _ => (),
                }
                debug!(action = "stderr"; "{buf}");
                lines.extend(buf.as_bytes().to_vec());
                lines.push(b'\n');
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
            Err(crate::Error::FailedToRunCommand(program))
        }
    }
}
