#[macro_use]
extern crate vst;

#[macro_use]
extern crate conrod_core;

use rand::random;
use std::os::raw::c_void;
use std::sync::Arc;
use vst::api::{Events, Supported};
use vst::buffer::AudioBuffer;
use vst::editor::Editor;
use vst::event::Event;
use vst::plugin::{CanDo, Category, Info, Plugin, PluginParameters};
use vst::util::AtomicFloat;

#[derive(Default)]
struct Whisper {
    params: Arc<WhisperParameters>,
    // Added a counter in our plugin struct.
    notes: u8,
}

struct WhisperParameters {
    volume: AtomicFloat,
}

impl Default for WhisperParameters {
    fn default() -> Self {
        Self {
            volume: AtomicFloat::new(1.0),
        }
    }
}

// We're implementing a trait `Plugin` that does all the VST-y stuff for us.
impl Plugin for Whisper {
    fn get_info(&self) -> Info {
        Info {
            name: "Whisper".to_string(),

            // Used by hosts to differentiate between plugins.
            unique_id: 1337,

            // We don't need inputs
            inputs: 0,

            // We do need two outputs though.  This is default, but let's be
            // explicit anyways.
            outputs: 2,

            // Set our category
            category: Category::Synth,

            parameters: 1,

            // We don't care about other stuff, and it can stay default.
            ..Default::default()
        }
    }

    // Here's the function that allows us to receive events
    fn process_events(&mut self, events: &Events) {
        // Some events aren't MIDI events - so let's do a match
        // to make sure we only get MIDI, since that's all we care about.
        for event in events.events() {
            match event {
                Event::Midi(ev) => {
                    // Check if it's a noteon or noteoff event.
                    // This is difficult to explain without knowing how the MIDI standard works.
                    // Basically, the first byte of data tells us if this signal is a note on event
                    // or a note off event.  You can read more about that here:
                    // https://www.midi.org/specifications/item/table-1-summary-of-midi-message
                    match ev.data[0] {
                        // if note on, increment our counter
                        144 => self.notes += 1u8,

                        // if note off, decrement our counter
                        128 => self.notes -= 1u8,
                        _ => (),
                    }
                    // if we cared about the pitch of the note, it's stored in `ev.data[1]`.
                }
                // We don't care if we get any other type of event
                _ => (),
            }
        }
    }

    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        // `buffer.split()` gives us a tuple containing the
        // input and output buffers.  We only care about the
        // output, so we can ignore the input by using `_`.
        let (_, mut output_buffer) = buffer.split();

        // We only want to process *anything* if a note is being held.
        // Else, we can fill the output buffer with silence.
        if self.notes == 0 {
            for output_channel in output_buffer.into_iter() {
                // Let's iterate over every sample in our channel.
                for output_sample in output_channel {
                    *output_sample = 0.0;
                }
            }
            return;
        }

        // Now, we want to loop over our output channels.  This
        // includes our left and right channels (or more, if you
        // are working with surround sound).
        for output_channel in output_buffer.into_iter() {
            // Let's iterate over every sample in our channel.
            for output_sample in output_channel {
                // For every sample, we want to generate a random value
                // from -1.0 to 1.0.
                *output_sample = (random::<f32>() - 0.5f32) * 2f32 * self.params.volume.get();
            }
        }
    }

    // It's good to tell our host what our plugin can do.
    // Some VST hosts might not send any midi events to our plugin
    // if we don't explicitly tell them that the plugin can handle them.
    fn can_do(&self, can_do: CanDo) -> Supported {
        match can_do {
            // Tell our host that the plugin supports receiving MIDI messages
            CanDo::ReceiveMidiEvent => Supported::Yes,
            // Maybe it also supports ather things
            _ => Supported::Maybe,
        }
    }

    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        Arc::clone(&self.params) as Arc<dyn PluginParameters>
    }

    fn get_editor(&mut self) -> Option<Box<dyn Editor>> {
        Some(Box::new(GUIWrapper::new(self.params.clone())))
    }
}

plugin_main!(Whisper);

impl PluginParameters for WhisperParameters {
    fn get_parameter_label(&self, index: i32) -> String {
        match index {
            0 => "x".to_string(),
            _ => "".to_string(),
        }
    }
    // This is what will display underneath our control.  We can
    // format it into a string that makes the most sense.
    fn get_parameter_text(&self, index: i32) -> String {
        match index {
            0 => format!("{:.3}", self.volume.get()),
            _ => format!(""),
        }
    }

    fn get_parameter_name(&self, index: i32) -> String {
        match index {
            0 => "volume".to_string(),
            _ => "".to_string(),
        }
    }
    // get_parameter has to return the value used in set_parameter
    fn get_parameter(&self, index: i32) -> f32 {
        match index {
            0 => self.volume.get(),
            _ => 0.0,
        }
    }
    fn set_parameter(&self, index: i32, value: f32) {
        match index {
            0 => self.volume.set(value),
            _ => (),
        }
    }
}

use winapi::shared::windef::HWND;
use winit::platform::desktop::EventLoopExtDesktop;
use winit::platform::windows::WindowBuilderExtWindows;

mod support;

use conrod_core::text::Font;
use conrod_core::{widget, Colorable, Positionable, Ui, Widget};
use conrod_glium::Renderer;
use glium::glutin::event_loop::EventLoop;
use glium::glutin::window::WindowBuilder;
use glium::Surface;
use winit::event_loop::ControlFlow;

const WIDTH: u32 = 400;
const HEIGHT: u32 = 200;

widget_ids!(struct Ids { text, volume_slider });

struct GUIWrapper {
    params: Arc<WhisperParameters>,
    inner: Option<GUI>,
}

struct GUI {
    event_loop: EventLoop<()>,
    display: support::GliumDisplayWinitWrapper,
    ids: Ids,
    ui: Ui,
    renderer: Renderer,
    image_map: conrod_core::image::Map<glium::texture::Texture2d>,
}

impl GUI {
    fn new(parent: HWND) -> Self {
        let event_loop = EventLoop::new();

        let window = WindowBuilder::new()
            .with_title("A fantastic window!")
            .with_decorations(false)
            .with_resizable(false)
            .with_parent_window(parent)
            .with_inner_size((WIDTH, HEIGHT).into());

        let context = glium::glutin::ContextBuilder::new();

        let display = glium::Display::new(window, context, &event_loop).unwrap();
        let display = support::GliumDisplayWinitWrapper(display);

        let mut ui = conrod_core::UiBuilder::new([WIDTH as f64, HEIGHT as f64]).build();
        let ids = Ids::new(ui.widget_id_generator());

        let font: &[u8] = include_bytes!("../assets/fonts/NotoSans/NotoSans-Regular.ttf");
        ui.fonts.insert(Font::from_bytes(font).unwrap());

        let renderer = conrod_glium::Renderer::new(&display.0).unwrap();

        // The image map describing each of our widget->image mappings (in our case, none).
        let image_map = conrod_core::image::Map::<glium::texture::Texture2d>::new();

        Self {
            event_loop,
            display,
            ids,
            ui,
            renderer,
            image_map,
        }
    }
}

impl GUIWrapper {
    fn new(params: Arc<WhisperParameters>) -> Self {
        Self {
            params,
            inner: None,
        }
    }
}

impl Editor for GUIWrapper {
    fn size(&self) -> (i32, i32) {
        if let Some(inner) = self.inner.as_ref() {
            let s = inner.display.0.gl_window().window().inner_size();
            (s.width as i32, s.height as i32)
        } else {
            (0, 0)
        }
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn idle(&mut self) {
        use winit::event;

        let mut end = false;
        if let Some(inner) = self.inner.as_mut() {
            let display = &mut inner.display;
            let ui = &mut inner.ui;
            let ids = &mut inner.ids;
            let renderer = &mut inner.renderer;
            let image_map = &mut inner.image_map;
            let params = &self.params;
            inner
                .event_loop
                .run_return(move |event, _, control_flow| match event {
                    event::Event::WindowEvent {
                        event: event::WindowEvent::CloseRequested,
                        window_id,
                    } if window_id == display.0.gl_window().window().id() => {
                        end = true;
                        *control_flow = ControlFlow::Exit
                    }
                    event::Event::EventsCleared => *control_flow = ControlFlow::Exit,
                    _ => {
                        let input = match support::convert_event(event, display) {
                            None => return,
                            Some(input) => input,
                        };

                        // Handle the input with the `Ui`.
                        ui.handle_event(input);

                        // Set the widgets.
                        let ui = &mut ui.set_widgets();

                        // "Hello World!" in the middle of the screen.
                        widget::Text::new("Volume")
                            .middle_of(ui.window)
                            .color(conrod_core::color::WHITE)
                            .font_size(32)
                            .set(ids.text, ui);

                        if let Some(new_volume) = widget::Slider::new(params.volume.get(), 0.0, 1.0)
                            .set(ids.volume_slider, ui)
                        {
                            params.volume.set(new_volume);
                        }

                        // Draw the `Ui` if it has changed.
                        if let Some(primitives) = ui.draw_if_changed() {
                            renderer.fill(&display.0, primitives, image_map);
                            let mut target = display.0.draw();
                            target.clear_color(0.0, 0.0, 0.0, 1.0);
                            renderer.draw(&display.0, &mut target, &image_map).unwrap();
                            target.finish().unwrap();
                        }
                    }
                });
        }
        if end {
            self.inner = None;
        }
    }

    fn close(&mut self) {
        self.inner = None;
    }

    fn open(&mut self, parent: *mut c_void) -> bool {
        self.inner = Some(GUI::new(parent as HWND));
        true
    }

    fn is_open(&mut self) -> bool {
        self.inner.is_some()
    }
}
