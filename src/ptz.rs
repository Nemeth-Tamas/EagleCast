use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use serde::Deserialize;

const PTZ_BASE_URL: &str = "http://192.168.1.201:8765";
const POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PtzStatus {
    Starting,
    Connected,
    Offline,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CameraState {
    pub pan_degrees: f64,
    pub tilt_degrees: f64,
    pub zoom_percent: f64,
}

struct SharedState {
    status: PtzStatus,
    camera: Option<CameraState>,
    error: Option<String>,
    last_update: Option<Instant>,
}

pub struct PtzSnapshot {
    pub status: PtzStatus,
    pub camera: Option<CameraState>,
    pub error: Option<String>,
    pub last_update: Option<Instant>,
}

pub struct PtzController {
    shared: Arc<Mutex<SharedState>>,
    stop: Arc<AtomicBool>,
}

impl PtzController {
    pub fn start() -> Self {
        let shared = Arc::new(Mutex::new(SharedState {
            status: PtzStatus::Starting,
            camera: None,
            error: None,
            last_update: None,
        }));

        let stop = Arc::new(AtomicBool::new(false));

        let worker_shared = Arc::clone(&shared);
        let worker_stop = Arc::clone(&stop);

        thread::Builder::new()
            .name("eaglecast-ptz".to_owned())
            .spawn(move || worker_loop(worker_shared, worker_stop))
            .expect("failed to start EagleCast PTZ worker");

        Self { shared, stop }
    }

    pub fn snapshot(&self) -> PtzSnapshot {
        let state = self
            .shared
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        PtzSnapshot {
            status: state.status,
            camera: state.camera.clone(),
            error: state.error.clone(),
            last_update: state.last_update,
        }
    }
}

impl Drop for PtzController {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn worker_loop(shared: Arc<Mutex<SharedState>>, stop: Arc<AtomicBool>) {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(750))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            set_error(&shared, format!("failed to create HTTP client: {error}"));
            return;
        }
    };

    let state_url = format!("{PTZ_BASE_URL}/api/state");

    while !stop.load(Ordering::Relaxed) {
        let result = client
            .get(&state_url)
            .send()
            .and_then(|response| response.error_for_status())
            .and_then(|response| response.json::<CameraState>());

        match result {
            Ok(camera) => {
                let mut state = shared
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());

                state.status = PtzStatus::Connected;
                state.camera = Some(camera);
                state.error = None;
                state.last_update = Some(Instant::now());
            }
            Err(error) => {
                set_error(&shared, format!("PTZ state request failed: {error}"));
            }
        }

        let sleep_steps = POLL_INTERVAL.as_millis() / 10;

        for _ in 0..sleep_steps {
            if stop.load(Ordering::Relaxed) {
                return;
            }

            thread::sleep(Duration::from_millis(10));
        }
    }
}

fn set_error(shared: &Arc<Mutex<SharedState>>, error: String) {
    let mut state = shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    state.status = PtzStatus::Offline;
    state.error = Some(error);
}
