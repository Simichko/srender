use std::error::Error;
use std::num::NonZeroU32;
use std::sync::Arc;

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, OwnedDisplayHandle};
use winit::window::{Window, WindowId};

#[derive(Debug)]
struct App {
    context: Context<OwnedDisplayHandle>,
    surface: Option<Surface<OwnedDisplayHandle, Arc<Window>>>,
    window: Option<Arc<Window>>,
    mouse_pos: PhysicalPosition<f64>,
    last_mouse_pos: Option<PhysicalPosition<f64>>,
    mouse_pressed: bool,
    canvas: Vec<u32>,
    canvas_width: u32,
    canvas_height: u32,
}

impl App {
    fn new(context: Context<OwnedDisplayHandle>) -> Self {
        App {
            context,
            surface: None,
            window: None,
            mouse_pos: PhysicalPosition { x: 0.0, y: 0.0 },
            last_mouse_pos: None,
            mouse_pressed: false,
            canvas: Vec::new(),
            canvas_width: 0,
            canvas_height: 0,
        }
    }

    fn draw(&mut self) {
        let surface = self.surface.as_mut().unwrap();
        let window = surface.window();

        window.pre_present_notify();
        let size = window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return;
        };

        // Fill a buffer with a solid color.
        const DARK_GRAY: u32 = 0xff181818;

        surface
            .resize(width, height)
            .expect("Failed to resize the softbuffer surface");

        let mut buffer = surface
            .buffer_mut()
            .expect("Failed to get the softbuffer buffer");

        buffer.fill(DARK_GRAY);

        for y in 0..100 {
            let row_start = buffer.width().get() * y;

            for x in 0..100 {
                buffer[(row_start + x) as usize] = 0xff;
            }
        }

        for y in 300..400 {
            let row_start = buffer.width().get() * y;

            for x in 100..500 {
                buffer[(row_start + x) as usize] = 0xff;
            }
        }

        buffer
            .present()
            .expect("Failed to present the softbuffer buffer");
    }

    fn paint(&mut self) {
        if self.mouse_pressed {
            let width = self.canvas_width;
            let height = self.canvas_height;

            let start = self.last_mouse_pos.unwrap_or(self.mouse_pos);
            let end = self.mouse_pos;

            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let steps = dx.abs().max(dy.abs()).ceil() as u32;
            let steps = steps.max(1);

            for i in 0..=steps {
                let t = i as f64 / steps as f64;
                let x = (start.x + dx * t) as u32;
                let y = (start.y + dy * t) as u32;

                for py in y..(y + 10).min(height) {
                    let row_start = width * py;
                    for px in x..(x + 10).min(width) {
                        self.canvas[(row_start + px) as usize] = 0xffffffff;
                    }
                }
            }

            self.last_mouse_pos = Some(end);
        }

        let surface = self.surface.as_mut().unwrap();
        let mut buffer = surface
            .buffer_mut()
            .expect("Failed to get the softbuffer buffer");

        buffer.copy_from_slice(&self.canvas);
        buffer
            .present()
            .expect("Failed to present the softbuffer buffer");
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("resumed");

        if !self.surface.is_none() {
            return;
        }

        let win_attr = Window::default_attributes();

        let window = event_loop.create_window(win_attr).unwrap();
        let window = Arc::new(window);

        let mut surface = Surface::new(&self.context, Arc::clone(&window))
            .expect("Failed to create a softbuffer surface");

        let size = window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return;
        };

        // Fill a buffer with a solid color.
        // const DARK_GRAY: u32 = 0xff181818;

        surface
            .resize(width, height)
            .expect("Failed to resize the softbuffer surface");

        self.canvas = vec![0; (width.get() * height.get()) as usize];
        self.canvas_width = width.get();
        self.canvas_height = height.get();

        self.surface = Some(surface);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                self.mouse_pos = position;

                if !self.mouse_pressed {
                    return;
                }

                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => {
                if button == MouseButton::Left && state.is_pressed() {
                    self.mouse_pressed = true;
                    self.last_mouse_pos = Some(self.mouse_pos);
                    self.window.as_ref().unwrap().request_redraw();
                    return;
                }

                self.mouse_pressed = false;
                self.last_mouse_pos = None;
            }
            WindowEvent::Resized(size) => {
                if let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                {
                    if let Some(surface) = self.surface.as_mut() {
                        surface.resize(width, height).expect("Failed to resize surface");
                    }
                    self.canvas = vec![0; (width.get() * height.get()) as usize];
                    self.canvas_width = width.get();
                    self.canvas_height = height.get();
                }
            }
            WindowEvent::RedrawRequested => {
                if self.surface.is_none() {
                    return;
                }

                self.window.as_ref().unwrap().pre_present_notify();
                self.paint();
            }
            _ => (),
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {}
}

#[derive(Debug, Clone, Copy)]
enum UserEvent {
    WakeUp,
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    // let event_loop_proxy = event_loop.create_proxy();

    let context = Context::new(event_loop.owned_display_handle()).unwrap();
    let mut app = App::new(context);

    // thread::spawn(move || {
    //     _ = event_loop_proxy.send_event(UserEvent::WakeUp);
    // });

    event_loop.run_app(&mut app)?;

    Ok(())
}
