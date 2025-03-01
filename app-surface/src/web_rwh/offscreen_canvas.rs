use super::{Canvas, SendSyncWrapper};
use core::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use wasm_bindgen::JsValue;

#[derive(Debug)]
pub struct OffscreenCanvasWrapper(SendSyncWrapper<OffscreenCanvas>);
impl OffscreenCanvasWrapper {
    pub fn new(canvas: OffscreenCanvas) -> Self {
        OffscreenCanvasWrapper(SendSyncWrapper(canvas))
    }
}

impl Deref for OffscreenCanvasWrapper {
    type Target = OffscreenCanvas;

    fn deref(&self) -> &Self::Target {
        &self.0.0
    }
}

impl DerefMut for OffscreenCanvasWrapper {
    fn deref_mut(&mut self) -> &mut OffscreenCanvas {
        &mut self.0.0
    }
}

#[derive(Debug, Clone)]
pub struct OffscreenCanvas {
    inner: web_sys::OffscreenCanvas,
    pub scale_factor: f32,
    handle: u32,
}

#[allow(dead_code)]
impl OffscreenCanvas {
    pub const fn new(canvas: web_sys::OffscreenCanvas, scale_factor: f32, handle: u32) -> Self {
        Self {
            inner: canvas,
            scale_factor,
            handle,
        }
    }

    pub fn each(self) -> (web_sys::OffscreenCanvas, u32) {
        (self.inner, self.handle)
    }

    pub fn logical_resolution(&self) -> (u32, u32) {
        let width = self.inner.width();
        let height = self.inner.height();
        (width, height)
    }
}

impl From<&Canvas> for OffscreenCanvas {
    fn from(value: &Canvas) -> Self {
        let offscreen = value.element.transfer_control_to_offscreen().unwrap();
        let handle = value.handle;
        Self::new(offscreen, value.scale_factor, handle)
    }
}

impl HasWindowHandle for OffscreenCanvasWrapper {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        use raw_window_handle::{RawWindowHandle, WebOffscreenCanvasWindowHandle, WindowHandle};

        let value: &JsValue = &self.inner;
        let obj: NonNull<core::ffi::c_void> = NonNull::from(value).cast();
        let handle = WebOffscreenCanvasWindowHandle::new(obj);
        let raw = RawWindowHandle::WebOffscreenCanvas(handle);
        unsafe { Ok(WindowHandle::borrow_raw(raw)) }
    }
}

impl HasDisplayHandle for OffscreenCanvasWrapper {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        use raw_window_handle::{DisplayHandle, RawDisplayHandle, WebDisplayHandle};
        let handle = WebDisplayHandle::new();
        let raw = RawDisplayHandle::Web(handle);
        unsafe { Ok(DisplayHandle::borrow_raw(raw)) }
    }
}
