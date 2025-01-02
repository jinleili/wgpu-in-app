use super::SendSyncWrapper;
use raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, RawWindowHandle, WebCanvasWindowHandle, WindowHandle,
};
use std::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};
use wasm_bindgen::{JsCast, JsValue};

#[derive(Debug)]
pub struct CanvasWrapper(SendSyncWrapper<Canvas>);
impl CanvasWrapper {
    pub fn new(canvas: Canvas) -> Self {
        CanvasWrapper(SendSyncWrapper(canvas))
    }
}

impl Deref for CanvasWrapper {
    type Target = Canvas;

    fn deref(&self) -> &Self::Target {
        &self.0 .0
    }
}

impl DerefMut for CanvasWrapper {
    fn deref_mut(&mut self) -> &mut Canvas {
        &mut self.0 .0
    }
}

#[derive(Debug, Clone)]
pub struct Canvas {
    pub(crate) element: web_sys::HtmlCanvasElement,
    pub scale_factor: f32,
    pub(crate) handle: u32,
}

#[allow(dead_code)]
impl Canvas {
    pub fn new(element_id: &str, handle: u32) -> Self {
        // 0 is reserved for window itself.
        assert!(handle > 0);

        // 添加 `raw-window-handle` 需要的属性/值.
        let (element, scale_factor) = Self::get_canvas_element(element_id);
        element
            .set_attribute("data-raw-handle", handle.to_string().as_str())
            .unwrap();

        Self {
            element,
            scale_factor,
            handle,
        }
    }

    pub fn get_canvas_element(element_id: &str) -> (web_sys::HtmlCanvasElement, f32) {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let element = document
            .get_element_by_id(element_id)
            .expect("页面中找不到 canvas 元素 ");

        let canvas = element.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
        let scale_factor = window.device_pixel_ratio() as f32;

        (canvas, scale_factor)
    }

    #[inline]
    pub fn handle(&self) -> u32 {
        self.handle
    }

    pub fn logical_resolution(&self) -> (u32, u32) {
        let width = self.element.width();
        let height = self.element.height();
        (width, height)
    }
}

impl Deref for Canvas {
    type Target = web_sys::HtmlCanvasElement;

    fn deref(&self) -> &Self::Target {
        &self.element
    }
}

impl HasWindowHandle for CanvasWrapper {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let value: &JsValue = &self.element;
        let obj: NonNull<std::ffi::c_void> = NonNull::from(value).cast();
        let handle = WebCanvasWindowHandle::new(obj);
        let raw = RawWindowHandle::WebCanvas(handle);
        unsafe { Ok(WindowHandle::borrow_raw(raw)) }
    }
}

impl HasDisplayHandle for CanvasWrapper {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        use raw_window_handle::{DisplayHandle, RawDisplayHandle, WebDisplayHandle};
        let handle = WebDisplayHandle::new();
        let raw = RawDisplayHandle::Web(handle);
        unsafe { Ok(DisplayHandle::borrow_raw(raw)) }
    }
}
