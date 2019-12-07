#![allow(dead_code)]

pub struct GliumDisplayWinitWrapper(pub glium::Display);

impl conrod_winit::WinitWindow for GliumDisplayWinitWrapper {
    fn get_inner_size(&self) -> Option<(u32, u32)> {
        Some(self.0.gl_window().window().inner_size().into())
    }
    fn hidpi_factor(&self) -> f32 {
        self.0.gl_window().window().hidpi_factor() as _
    }
}

// Conversion functions for converting between types from glium's version of `winit` and
// `conrod_core`.
conrod_winit::conversion_fns!();
