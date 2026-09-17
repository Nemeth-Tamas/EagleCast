use std::{
    io::Read,
    path::Path,
    process::{Child, ChildStdout, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub const FRAME_WIDTH: usize = 1280;
pub const FRAME_HEIGHT: usize = 720;

const FRAME_BYTES: usize = FRAME_WIDTH * FRAME_HEIGHT * 4;
const FFMPEG_PATH: &str = r"F:\ffmpeg-master-latest-win64-gpl\bin\ffmpeg.exe";
const STREAM_URL: &str = "udp://@239.42.0.1:5000?fifo_size=1000000&overrun_nonfatal=1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamStatus {
    Starting,
    Live,
    Offline,
}

pub struct VideoFrame {
    pub rgba: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub sequence: u64,
    pub received_at: Instant,
}

struct SharedState {
    status: StreamStatus,
    error: Option<String>,
    latest_frame: Option<Arc<VideoFrame>>,
    frames_received: u64,
}

pub struct StreamSnapshot {
    pub status: StreamStatus,
    pub error: Option<String>,
    pub latest_frame: Option<Arc<VideoFrame>>,
    pub frames_received: u64,
}

pub struct StreamController {
    shared: Arc<Mutex<SharedState>>,
    stop: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
}

impl StreamController {
    pub fn start() -> Self {
        let shared = Arc::new(Mutex::new(SharedState {
            status: StreamStatus::Starting,
            error: None,
            latest_frame: None,
            frames_received: 0,
        }));

        let stop = Arc::new(AtomicBool::new(false));
        let child = Arc::new(Mutex::new(None));

        let worker_shared = Arc::clone(&shared);
        let worker_stop = Arc::clone(&stop);
        let worker_child = Arc::clone(&child);

        thread::Builder::new()
            .name("eaglecast-video".to_owned())
            .spawn(move || worker_loop(worker_shared, worker_stop, worker_child))
            .expect("failed to start EagleCast video worker");

        Self {
            shared,
            stop,
            child,
        }
    }

    pub fn snapshot(&self) -> StreamSnapshot {
        let state = self
            .shared
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        StreamSnapshot {
            status: state.status,
            error: state.error.clone(),
            latest_frame: state.latest_frame.clone(),
            frames_received: state.frames_received,
        }
    }
}

impl Drop for StreamController {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);

        let mut child_slot = self
            .child
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(child) = child_slot.as_mut() {
            let _ = child.kill();
        }
    }
}

fn worker_loop(
    shared: Arc<Mutex<SharedState>>,
    stop: Arc<AtomicBool>,
    child_slot: Arc<Mutex<Option<Child>>>,
) {
    while !stop.load(Ordering::Relaxed) {
        set_status(&shared, StreamStatus::Starting, None);

        match spawn_ffmpeg() {
            Ok((child, mut stdout)) => {
                {
                    let mut slot = child_slot
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());

                    *slot = Some(child);
                }

                let read_result = read_frames(&mut stdout, &shared, &stop);

                {
                    let mut slot = child_slot
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());

                    if let Some(mut child) = slot.take() {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }

                if stop.load(Ordering::Relaxed) {
                    break;
                }

                let error = read_result
                    .err()
                    .unwrap_or_else(|| "FFmpeg stopped producing video".to_owned());

                set_status(&shared, StreamStatus::Offline, Some(error));
            }
            Err(error) => {
                set_status(&shared, StreamStatus::Offline, Some(error));
            }
        }

        for _ in 0..10 {
            if stop.load(Ordering::Relaxed) {
                return;
            }

            thread::sleep(Duration::from_millis(100));
        }
    }
}

fn spawn_ffmpeg() -> Result<(Child, ChildStdout), String> {
    if !Path::new(FFMPEG_PATH).exists() {
        return Err(format!("FFmpeg was not found at {FFMPEG_PATH}"));
    }

    let mut command = Command::new(FFMPEG_PATH);

    command
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-fflags",
            "nobuffer",
            "-flags",
            "low_delay",
            "-analyzeduration",
            "0",
            "-probesize",
            "32",
            "-i",
            STREAM_URL,
            "-an",
            "-vf",
            "scale=1280:720:flags=fast_bilinear",
            "-pix_fmt",
            "rgba",
            "-f",
            "rawvideo",
            "pipe:1",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to start FFmpeg: {error}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "FFmpeg stdout pipe was not available".to_owned())?;

    Ok((child, stdout))
}

fn read_frames(
    stdout: &mut ChildStdout,
    shared: &Arc<Mutex<SharedState>>,
    stop: &Arc<AtomicBool>,
) -> Result<(), String> {
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }

        let mut rgba = vec![0_u8; FRAME_BYTES];

        stdout
            .read_exact(&mut rgba)
            .map_err(|error| format!("video pipe ended: {error}"))?;

        let mut state = shared
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        state.frames_received = state.frames_received.saturating_add(1);

        let sequence = state.frames_received;

        state.latest_frame = Some(Arc::new(VideoFrame {
            rgba,
            width: FRAME_WIDTH,
            height: FRAME_HEIGHT,
            sequence,
            received_at: Instant::now(),
        }));

        state.status = StreamStatus::Live;
        state.error = None;
    }
}

fn set_status(shared: &Arc<Mutex<SharedState>>, status: StreamStatus, error: Option<String>) {
    let mut state = shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    state.status = status;
    state.error = error;
}
