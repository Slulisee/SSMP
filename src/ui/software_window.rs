//! Cached CPU painting and a native software surface. No GPU device or swapchain.
use super::*;
use egui_software_backend::{BufferMutRef, ColorFieldOrder, EguiSoftwareRender};
use std::{num::NonZeroU32, sync::Arc};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

struct Runtime {
    ctx: egui::Context,
    viewport: egui::ViewportBuilder,
    root: PathBuf,
    smoke: bool,
    page: Option<String>,
    app: Option<App>,
    window: Option<Arc<Window>>,
    input: Option<egui_winit::State>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    display: Option<softbuffer::Context<Arc<Window>>>,
    renderer: EguiSoftwareRender,
    next: Option<Instant>,
    last_paint: Instant,
    pending: Vec<egui::Event>,
    error: Option<String>,
    frames: u64,
    work_seconds: f64,
    started: Instant,
    interactive_until: Instant,
    background_next: Instant,
    background_ticks: u64,
}

pub(super) fn run(
    root: PathBuf,
    smoke: bool,
    page: Option<String>,
    viewport: egui::ViewportBuilder,
) -> Result<()> {
    let event_loop = EventLoop::<(Instant, u64)>::with_user_event().build()?;
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let proxy = event_loop.create_proxy();
    ctx.set_request_repaint_callback(move |info| {
        if let Some(when) = Instant::now().checked_add(info.delay) {
            let _ = proxy.send_event((when, info.current_cumulative_pass_nr));
        }
    });
    let mut runtime = Runtime {
        ctx,
        viewport,
        root,
        smoke,
        page,
        app: None,
        window: None,
        input: None,
        surface: None,
        display: None,
        renderer: EguiSoftwareRender::new(ColorFieldOrder::Bgra),
        next: None,
        last_paint: Instant::now() - Duration::from_secs(1),
        pending: vec![],
        error: None,
        frames: 0,
        work_seconds: 0.,
        started: Instant::now(),
        interactive_until: Instant::now(),
        background_next: Instant::now(),
        background_ticks: 0,
    };
    event_loop.run_app(&mut runtime)?;
    if let Some(error) = runtime.error {
        anyhow::bail!(error);
    }
    Ok(())
}

impl Runtime {
    fn schedule(&mut self, when: Instant) {
        self.next = Some(self.next.map_or(when, |next| next.min(when)));
    }
    fn close(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(mut app) = self.app.take() {
            app.close_window(None);
        }
        event_loop.exit();
    }
    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let window = Arc::new(egui_winit::create_window(
            &self.ctx,
            event_loop,
            &self.viewport,
        )?);
        let display =
            softbuffer::Context::new(window.clone()).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let surface = softbuffer::Surface::new(&display, window.clone())
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let mut input = egui_winit::State::new(
            self.ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            window.theme(),
            None,
        );
        egui_winit::update_viewport_info(
            input
                .egui_input_mut()
                .viewports
                .entry(egui::ViewportId::ROOT)
                .or_default(),
            &self.ctx,
            &window,
            true,
        );
        self.app = Some(App::new_context(
            &self.ctx,
            None,
            None,
            self.root.clone(),
            self.smoke,
            self.page.clone(),
        )?);
        self.input = Some(input);
        self.surface = Some(surface);
        self.display = Some(display);
        window.request_redraw();
        self.window = Some(window);
        Ok(())
    }
    fn paint(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let start = Instant::now();
        // Input bursts and hover animations cannot create an uncapped paint loop.
        let frame_budget = if start < self.interactive_until {
            Duration::from_micros(16_667)
        } else if self
            .app
            .as_ref()
            .is_some_and(|app| app.page == Page::NowPlaying)
        {
            Duration::from_millis(33)
        } else {
            Duration::from_millis(40)
        };
        let earliest = self.last_paint + frame_budget;
        if start < earliest {
            self.schedule(earliest);
            return Ok(());
        }
        self.last_paint = start;
        self.next = None;
        let window = self.window.as_ref().unwrap().clone();
        if window.is_minimized() == Some(true) {
            return Ok(());
        }
        let size = window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return Ok(());
        };
        let input = self.input.as_mut().unwrap();
        egui_winit::update_viewport_info(
            input
                .egui_input_mut()
                .viewports
                .entry(egui::ViewportId::ROOT)
                .or_default(),
            &self.ctx,
            &window,
            false,
        );
        let mut raw = input.take_egui_input(&window);
        raw.events.append(&mut self.pending);
        let output = self.ctx.run(raw, |ctx| {
            self.app.as_mut().unwrap().update_window(ctx, None)
        });
        input.handle_platform_output(&window, output.platform_output);
        let mut actions = vec![];
        let mut close = false;
        if let Some(viewport) = output.viewport_output.get(&egui::ViewportId::ROOT) {
            close = viewport
                .commands
                .iter()
                .any(|c| matches!(c, egui::ViewportCommand::Close))
                && !viewport
                    .commands
                    .iter()
                    .any(|c| matches!(c, egui::ViewportCommand::CancelClose));
            let info = input
                .egui_input_mut()
                .viewports
                .entry(egui::ViewportId::ROOT)
                .or_default();
            egui_winit::process_viewport_commands(
                &self.ctx,
                info,
                viewport.commands.clone(),
                &window,
                &mut actions,
            );
            if let Some(when) = Instant::now().checked_add(viewport.repaint_delay) {
                self.schedule(when);
            }
        }
        let primitives = self.ctx.tessellate(output.shapes, output.pixels_per_point);
        let surface = self.surface.as_mut().unwrap();
        surface
            .resize(width, height)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let mut buffer = surface
            .buffer_mut()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        buffer.fill(0);
        self.renderer.render(
            &mut BufferMutRef::new(
                bytemuck::cast_slice_mut(&mut buffer),
                width.get() as usize,
                height.get() as usize,
            ),
            &primitives,
            &output.textures_delta,
            output.pixels_per_point,
        );
        for action in actions {
            match action {
                egui_winit::ActionRequested::Screenshot(user_data) => {
                    let pixels = buffer
                        .iter()
                        .map(|p| Color32::from_rgb((p >> 16) as u8, (p >> 8) as u8, *p as u8))
                        .collect();
                    self.pending.push(egui::Event::Screenshot {
                        viewport_id: egui::ViewportId::ROOT,
                        image: Arc::new(egui::ColorImage::new(
                            [width.get() as usize, height.get() as usize],
                            pixels,
                        )),
                        user_data,
                    });
                }
                egui_winit::ActionRequested::Cut => self.pending.push(egui::Event::Cut),
                egui_winit::ActionRequested::Copy => self.pending.push(egui::Event::Copy),
                egui_winit::ActionRequested::Paste => {
                    if let Some(text) = self.input.as_mut().unwrap().clipboard_text() {
                        self.pending.push(egui::Event::Paste(text));
                    }
                }
            }
        }
        buffer
            .present()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        self.frames += 1;
        self.work_seconds += start.elapsed().as_secs_f64();
        if !self.pending.is_empty() {
            self.schedule(Instant::now());
        }
        if std::env::var_os("SSMP_PROFILE").is_some() && self.frames % 25 == 0 {
            let proof = serde_json::json!({"backend":"Software", "frames":self.frames, "elapsed":self.started.elapsed().as_secs_f64(),"paint_work_seconds":self.work_seconds,"mean_paint_ms":1000.*self.work_seconds/self.frames as f64});
            let _ = std::fs::write(
                self.root.join("performance.json"),
                serde_json::to_vec_pretty(&proof).unwrap_or_default(),
            );
        }
        if close {
            self.close(event_loop);
        }
        Ok(())
    }
}

impl ApplicationHandler<(Instant, u64)> for Runtime {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            if let Err(error) = self.initialize(event_loop) {
                self.error = Some(format!("{error:#}"));
                self.close(event_loop);
            }
        }
    }
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, (when, pass): (Instant, u64)) {
        // Requests made during a completed pass are already represented by
        // viewport_output.repaint_delay. Replaying them causes duplicate paints.
        if pass >= self.ctx.cumulative_pass_nr() {
            self.schedule(when);
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.window.as_ref().is_none_or(|window| window.id() != id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => self.close(event_loop),
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.paint(event_loop) {
                    self.error = Some(format!("{error:#}"));
                    self.close(event_loop);
                }
            }
            event => {
                if matches!(
                    event,
                    WindowEvent::CursorMoved { .. }
                        | WindowEvent::MouseInput { .. }
                        | WindowEvent::MouseWheel { .. }
                        | WindowEvent::KeyboardInput { .. }
                ) {
                    self.interactive_until = Instant::now() + Duration::from_millis(250);
                }
                let response = self
                    .input
                    .as_mut()
                    .unwrap()
                    .on_window_event(self.window.as_ref().unwrap(), &event);
                if response.repaint {
                    self.schedule(Instant::now());
                }
            }
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if self
            .window
            .as_ref()
            .is_some_and(|window| window.is_minimized() == Some(true))
        {
            // Minimized windows may suppress redraw events. Service playback
            // independently of painting, below the listening clock's gap limit.
            self.next = None;
            if now >= self.background_next {
                if let Some(app) = &mut self.app {
                    app.process_events(&self.ctx);
                    app.tick();
                    self.background_ticks += 1;
                    if std::env::var_os("SSMP_PROFILE").is_some() && self.background_ticks % 4 == 0
                    {
                        let proof = serde_json::json!({
                            "ticks": self.background_ticks,
                            "paint_frames": self.frames,
                            "position": app.player.as_ref().map(|p| p.snapshot.position),
                            "listening_seconds": app.state.listening_seconds.values().sum::<f64>(),
                        });
                        let _ = std::fs::write(
                            self.root.join("background.json"),
                            serde_json::to_vec_pretty(&proof).unwrap_or_default(),
                        );
                    }
                }
                self.background_next = Instant::now() + Duration::from_millis(250);
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.background_next));
            return;
        }
        self.background_next = now;
        if let Some(when) = self.next {
            if when <= Instant::now() {
                self.next = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                event_loop.set_control_flow(ControlFlow::Wait);
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(when));
            }
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}
