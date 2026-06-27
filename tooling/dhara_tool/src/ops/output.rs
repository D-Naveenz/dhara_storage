use std::process::Child;
use std::sync::{Arc, Mutex, mpsc::Sender};

use once_cell::sync::Lazy;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputEvent {
    pub stream: OutputStream,
    pub line: String,
}

static OUTPUT_SENDER: Lazy<Mutex<Option<Sender<OutputEvent>>>> = Lazy::new(|| Mutex::new(None));
static ACTIVE_CHILD: Lazy<Mutex<Option<Arc<Mutex<Child>>>>> = Lazy::new(|| Mutex::new(None));

pub struct OutputCaptureGuard {
    previous_sender: Option<Sender<OutputEvent>>,
}

impl OutputCaptureGuard {
    pub fn install(sender: Sender<OutputEvent>) -> Self {
        let mut slot = OUTPUT_SENDER
            .lock()
            .expect("output sender mutex should not be poisoned");
        let previous_sender = slot.replace(sender);
        drop(slot);
        Self { previous_sender }
    }
}

impl Drop for OutputCaptureGuard {
    fn drop(&mut self) {
        let mut slot = OUTPUT_SENDER
            .lock()
            .expect("output sender mutex should not be poisoned");
        *slot = self.previous_sender.take();
    }
}

pub fn emit_stdout_line(line: impl Into<String>) {
    emit(OutputStream::Stdout, line.into());
}

pub fn emit_stderr_line(line: impl Into<String>) {
    emit(OutputStream::Stderr, line.into());
}

pub fn set_active_child(child: Option<Arc<Mutex<Child>>>) {
    let mut slot = ACTIVE_CHILD
        .lock()
        .expect("active child mutex should not be poisoned");
    *slot = child;
}

pub fn cancel_active_subprocess() -> bool {
    let child = {
        let slot = ACTIVE_CHILD
            .lock()
            .expect("active child mutex should not be poisoned");
        slot.clone()
    };
    let Some(child) = child else {
        return false;
    };

    child
        .lock()
        .expect("active child handle mutex should not be poisoned")
        .kill()
        .is_ok()
}

fn emit(stream: OutputStream, line: String) {
    let sender = {
        let slot = OUTPUT_SENDER
            .lock()
            .expect("output sender mutex should not be poisoned");
        slot.clone()
    };

    if let Some(sender) = sender {
        let _ = sender.send(OutputEvent { stream, line });
        return;
    }

    match stream {
        OutputStream::Stdout => println!("{line}"),
        OutputStream::Stderr => eprintln!("{line}"),
    }
}
