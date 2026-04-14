use std::sync::{Mutex, OnceLock};
use std::thread::JoinHandle;

use tokio::sync::oneshot;

use crate::FrameSender;

pub struct ServerState {
    pub frame_tx: Option<FrameSender>,
    pub shutdown_tx: Option<oneshot::Sender<()>>,
    pub worker: Option<JoinHandle<()>>,
}

impl ServerState {
    pub fn stopped() -> Self {
        Self {
            frame_tx: None,
            shutdown_tx: None,
            worker: None,
        }
    }
}

static SERVER_STATE: OnceLock<Mutex<ServerState>> = OnceLock::new();

pub fn state() -> &'static Mutex<ServerState> {
    SERVER_STATE.get_or_init(|| Mutex::new(ServerState::stopped()))
}
