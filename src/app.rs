use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, ColorImage, Rect, Sense, TextureHandle, TextureOptions, Vec2};

use crate::{
    ptz::{PtzController, PtzSnapshot, PtzStatus},
    stream::{FRAME_HEIGHT, FRAME_WIDTH, StreamController, StreamSnapshot, StreamStatus},
};

pub struct EagleCastApp {
    stream: StreamController,
    ptz: PtzController,
    texture: Option<TextureHandle>,
    roll_degrees: f32,
    last_sequence: u64,
    fps_window_started: Instant,
    fps_window_sequence: u64,
    presented_in_window: u64,
    input_fps: f32,
    output_fps: f32,
}

impl EagleCastApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            stream: StreamController::start(),
            ptz: PtzController::start(),
            texture: None,
            roll_degrees: 0.0,
            last_sequence: 0,
            fps_window_started: Instant::now(),
            fps_window_sequence: 0,
            presented_in_window: 0,
            input_fps: 0.0,
            output_fps: 0.0,
        }
    }

    fn update_texture(&mut self, ctx: &egui::Context, snapshot: &StreamSnapshot) {
        let Some(frame) = snapshot.latest_frame.as_ref() else {
            return;
        };

        if frame.sequence == self.last_sequence {
            return;
        }

        let image = ColorImage::from_rgba_unmultiplied([frame.width, frame.height], &frame.rgba);

        if let Some(texture) = self.texture.as_mut() {
            texture.set(image, TextureOptions::LINEAR);
        } else {
            self.texture = Some(ctx.load_texture("eagleeye-live", image, TextureOptions::LINEAR));
        }

        self.last_sequence = frame.sequence;
        self.presented_in_window = self.presented_in_window.saturating_add(1);
    }

    fn update_fps(&mut self, frames_received: u64) {
        let elapsed = self.fps_window_started.elapsed();

        if elapsed < Duration::from_secs(1) {
            return;
        }

        let seconds = elapsed.as_secs_f32();
        let input_frames = frames_received.saturating_sub(self.fps_window_sequence);

        self.input_fps = input_frames as f32 / seconds;
        self.output_fps = self.presented_in_window as f32 / seconds;

        self.fps_window_started = Instant::now();
        self.fps_window_sequence = frames_received;
        self.presented_in_window = 0;
    }

    fn draw_video(&self, ui: &mut egui::Ui) {
        let Some(texture) = self.texture.as_ref() else {
            ui.centered_and_justified(|ui| {
                ui.label("Waiting for EagleEye video...");
            });

            return;
        };

        let available = ui.available_rect_before_wrap();
        let aspect = FRAME_WIDTH as f32 / FRAME_HEIGHT as f32;
        let video_size = fit_aspect(available.size(), aspect);

        let video_rect = Rect::from_center_size(available.center(), video_size);

        ui.allocate_rect(video_rect, Sense::hover());

        let painter = ui.painter().with_clip_rect(video_rect);

        painter.rect_filled(video_rect, 0.0, Color32::BLACK);

        let angle = self.roll_degrees.to_radians();
        let crop_scale = cover_scale(self.roll_degrees);
        let half = video_rect.size() * (0.5 * crop_scale);
        let center = video_rect.center();

        let cos = angle.cos();
        let sin = angle.sin();

        let local_corners = [
            egui::vec2(-half.x, -half.y),
            egui::vec2(half.x, -half.y),
            egui::vec2(half.x, half.y),
            egui::vec2(-half.x, half.y),
        ];

        let uvs = [
            egui::pos2(0.0, 0.0),
            egui::pos2(1.0, 0.0),
            egui::pos2(1.0, 1.0),
            egui::pos2(0.0, 1.0),
        ];

        let mut mesh = egui::Mesh::with_texture(texture.id());

        for (local, uv) in local_corners.into_iter().zip(uvs) {
            let rotated = egui::vec2(local.x * cos - local.y * sin, local.x * sin + local.y * cos);

            mesh.vertices.push(egui::epaint::Vertex {
                pos: center + rotated,
                uv,
                color: Color32::WHITE,
            });
        }

        mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);

        painter.add(egui::Shape::mesh(mesh));
    }

    fn draw_ptz(&self, ui: &mut egui::Ui, snapshot: &PtzSnapshot) {
        ui.heading("PTZ position");

        match snapshot.status {
            PtzStatus::Starting => {
                ui.label(egui::RichText::new("API STARTING").strong());
            }
            PtzStatus::Connected => {
                ui.label(egui::RichText::new("API CONNECTED").strong());
            }
            PtzStatus::Offline => {
                ui.label(egui::RichText::new("API OFFLINE").strong());
            }
        }

        if let Some(camera) = snapshot.camera.as_ref() {
            ui.label(format!("Pan:   {:.1} deg", camera.pan_degrees));
            ui.label(format!("Tilt:  {:.1} deg", camera.tilt_degrees));
            ui.label(format!("Zoom:  {:.1} %", camera.zoom_percent));
        } else {
            ui.label("Pan:   --");
            ui.label("Tilt:  --");
            ui.label("Zoom:  --");
        }

        if let Some(last_update) = snapshot.last_update {
            ui.label(format!(
                "State age: {} ms",
                last_update.elapsed().as_millis()
            ));
        }

        if let Some(error) = snapshot.error.as_ref() {
            ui.add_space(4.0);
            ui.label(error);
        }
    }
}

impl eframe::App for EagleCastApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let stream_snapshot = self.stream.snapshot();
        let ptz_snapshot = self.ptz.snapshot();

        self.update_texture(ctx, &stream_snapshot);
        self.update_fps(stream_snapshot.frames_received);

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("EagleCast");
                ui.separator();
                ui.label("Polycom EagleEye live processor");
            });
        });

        egui::SidePanel::right("controls")
            .resizable(false)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Video");

                match stream_snapshot.status {
                    StreamStatus::Starting => {
                        ui.label(egui::RichText::new("STREAM STARTING").strong());
                    }
                    StreamStatus::Live => {
                        ui.label(egui::RichText::new("STREAM LIVE").strong());
                    }
                    StreamStatus::Offline => {
                        ui.label(egui::RichText::new("STREAM OFFLINE").strong());
                    }
                }

                ui.label(format!("Decoded: {:.1} FPS", self.input_fps));

                ui.label(format!("Presented: {:.1} FPS", self.output_fps));

                ui.label(format!("Working size: {} x {}", FRAME_WIDTH, FRAME_HEIGHT));

                if let Some(frame) = stream_snapshot.latest_frame.as_ref() {
                    ui.label(format!(
                        "Latest frame age: {} ms",
                        frame.received_at.elapsed().as_millis()
                    ));
                }

                if let Some(error) = stream_snapshot.error.as_ref() {
                    ui.add_space(4.0);
                    ui.label(error);
                }

                ui.add_space(12.0);
                ui.separator();

                self.draw_ptz(ui, &ptz_snapshot);

                ui.add_space(12.0);
                ui.separator();

                ui.heading("Stabilization");

                ui.add(
                    egui::Slider::new(&mut self.roll_degrees, -30.0..=30.0)
                        .text("Roll")
                        .suffix(" deg")
                        .step_by(0.1),
                );

                ui.label(format!(
                    "Auto-crop scale: {:.3}x",
                    cover_scale(self.roll_degrees)
                ));

                if ui.button("Reset roll").clicked() {
                    self.roll_degrees = 0.0;
                }
            });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Frames decoded: {}",
                    stream_snapshot.frames_received
                ));

                ui.separator();

                ui.label(format!("Roll: {:.1} deg", self.roll_degrees));

                ui.separator();

                if let Some(camera) = ptz_snapshot.camera.as_ref() {
                    ui.label(format!(
                        "PTZ: P {:.1} / T {:.1} / Z {:.1}%",
                        camera.pan_degrees, camera.tilt_degrees, camera.zoom_percent
                    ));
                } else {
                    ui.label("PTZ: unavailable");
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_video(ui);
        });

        ctx.request_repaint_after(Duration::from_millis(8));
    }
}

fn fit_aspect(available: Vec2, aspect: f32) -> Vec2 {
    if available.x <= 0.0 || available.y <= 0.0 {
        return Vec2::ZERO;
    }

    if available.x / available.y > aspect {
        egui::vec2(available.y * aspect, available.y)
    } else {
        egui::vec2(available.x, available.x / aspect)
    }
}

fn cover_scale(roll_degrees: f32) -> f32 {
    let angle = roll_degrees.to_radians().abs();
    let cos = angle.cos().abs();
    let sin = angle.sin().abs();
    let aspect = FRAME_WIDTH as f32 / FRAME_HEIGHT as f32;

    (cos + aspect * sin).max(cos + sin / aspect)
}
