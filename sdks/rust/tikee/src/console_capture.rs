use std::{future::Future, io::Read, sync::Arc};

use os_pipe::pipe;
use stdio_override::{StderrOverride, StdoutOverride};
use tokio::sync::Mutex;

use crate::error::WorkerSdkError;

static CAPTURE_LOCK: Mutex<()> = Mutex::const_new(());

pub async fn capture_task_console<F, Fut, T, E>(
    emit_log: Arc<E>,
    run: F,
) -> Result<T, WorkerSdkError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, WorkerSdkError>>,
    E: Fn(&str, String) + Send + Sync + 'static,
{
    let _guard = CAPTURE_LOCK.lock().await;
    let (stdout_reader, stdout_writer) =
        pipe().map_err(|error| WorkerSdkError::ConsoleCaptureFailed(error.to_string()))?;
    let (stderr_reader, stderr_writer) =
        pipe().map_err(|error| WorkerSdkError::ConsoleCaptureFailed(error.to_string()))?;
    let stdout_guard = StdoutOverride::from_io_ref(&stdout_writer)
        .map_err(|error| WorkerSdkError::ConsoleCaptureFailed(error.to_string()))?;
    let stderr_guard = StderrOverride::from_io_ref(&stderr_writer)
        .map_err(|error| WorkerSdkError::ConsoleCaptureFailed(error.to_string()))?;

    let stdout_emit = Arc::clone(&emit_log);
    let stdout_handle = std::thread::spawn(move || {
        mirror_stream(stdout_reader, "info", move |level, message| {
            stdout_emit(level, message);
        })
    });
    let stderr_emit = Arc::clone(&emit_log);
    let stderr_handle = std::thread::spawn(move || {
        mirror_stream(stderr_reader, "error", move |level, message| {
            stderr_emit(level, message);
        })
    });

    let result = run().await;
    drop(stdout_guard);
    drop(stderr_guard);
    drop(stdout_writer);
    drop(stderr_writer);

    let stdout_output = stdout_handle.join().map_err(|_| {
        WorkerSdkError::ConsoleCaptureFailed("stdout capture thread panicked".to_owned())
    })??;
    let stderr_output = stderr_handle.join().map_err(|_| {
        WorkerSdkError::ConsoleCaptureFailed("stderr capture thread panicked".to_owned())
    })??;
    if !stdout_output.is_empty() {
        print!("{}", String::from_utf8_lossy(&stdout_output));
    }
    if !stderr_output.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&stderr_output));
    }

    result
}

fn mirror_stream<E>(
    mut reader: os_pipe::PipeReader,
    level: &'static str,
    emit_log: E,
) -> Result<Vec<u8>, WorkerSdkError>
where
    E: Fn(&str, String),
{
    let mut output = Vec::<u8>::new();
    let mut pending = Vec::<u8>::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| WorkerSdkError::ConsoleCaptureFailed(error.to_string()))?;
        if count == 0 {
            break;
        }
        let chunk = &buffer[..count];
        output.extend_from_slice(chunk);
        for value in chunk {
            match *value {
                b'\n' => emit_line(level, &mut pending, &emit_log),
                b'\r' => {}
                other => pending.push(other),
            }
        }
    }
    if !pending.is_empty() {
        emit_line(level, &mut pending, &emit_log);
    }
    Ok(output)
}

fn emit_line<E>(level: &str, pending: &mut Vec<u8>, emit_log: &E)
where
    E: Fn(&str, String),
{
    let line = String::from_utf8_lossy(pending).trim().to_owned();
    pending.clear();
    if !line.is_empty() {
        emit_log(level, line);
    }
}
