use std::sync::{Arc, mpsc};

use mio::Waker;

use crate::{DaemonError, DaemonEvent};

#[derive(Clone, Copy)]
pub(super) enum ControlMessage {
    Reload,
    Shutdown,
}

#[derive(Debug, Clone)]
pub(super) struct ControlHandle {
    sender: mpsc::Sender<ControlMessage>,
    waker: Arc<Waker>,
}

impl ControlHandle {
    pub(super) const fn new(sender: mpsc::Sender<ControlMessage>, waker: Arc<Waker>) -> Self {
        Self { sender, waker }
    }

    pub(super) fn reload(&self) -> Result<(), DaemonError> {
        self.sender
            .send(ControlMessage::Reload)
            .map_err(|_source| DaemonError::ControlChannelClosed)?;
        self.wake()
    }

    pub(super) fn shutdown(&self) -> Result<(), DaemonError> {
        self.sender
            .send(ControlMessage::Shutdown)
            .map_err(|_source| DaemonError::ControlChannelClosed)?;
        self.wake()
    }

    fn wake(&self) -> Result<(), DaemonError> {
        self.waker
            .wake()
            .map_err(|source| DaemonError::PollWake { source })
    }
}

pub(super) fn collect_controls(
    receiver: &mpsc::Receiver<ControlMessage>,
    controls: &mut Vec<ControlMessage>,
) {
    while let Ok(message) = receiver.try_recv() {
        controls.push(message);
    }
}

pub(super) fn emit_controls(
    controls: Vec<ControlMessage>,
    emit: &mut impl FnMut(DaemonEvent) -> Result<(), DaemonError>,
) -> Result<bool, DaemonError> {
    let mut running = true;
    for message in controls {
        running = emit_control(message, emit)?;
        if !running {
            return Ok(false);
        }
    }
    Ok(running)
}

fn emit_control(
    message: ControlMessage,
    emit: &mut impl FnMut(DaemonEvent) -> Result<(), DaemonError>,
) -> Result<bool, DaemonError> {
    match message {
        ControlMessage::Reload => {
            emit(DaemonEvent::ReloadRequested)?;
            Ok(true)
        }
        ControlMessage::Shutdown => {
            emit(DaemonEvent::ShutdownRequested)?;
            Ok(false)
        }
    }
}
