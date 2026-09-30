//! Minimal Windows platform layer: opens a window, reads keyboard input,
//! and copies our framebuffer onto the screen.

use crate::framebuffer::Framebuffer;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

/// Windows identifies windows, device contexts, etc. with opaque pointers.
type Handle = *mut c_void;

/// Virtual-key codes for the keys we care about.
pub mod key {
    pub const ESCAPE: usize = 0x1B;
    pub const F12: usize = 0x7B;
    pub const N: usize = 0x4E;
}

// Message and flag values, copied from the Windows headers.
const WM_DESTROY: u32 = 0x0002;
const WM_QUIT: u32 = 0x0012;
const WM_KEYDOWN: u32 = 0x0100;
const WM_SYSKEYDOWN: u32 = 0x0104;
const PM_REMOVE: u32 = 0x0001;
const CS_OWNDC: u32 = 0x0020;
const WS_VISIBLE: u32 = 0x1000_0000;
/// Title bar + close button + minimize button, but not resizable.
const WS_FIXED_SIZE_WINDOW: u32 = 0x00CA_0000;
const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;
const IDC_ARROW: usize = 32512;
const BI_RGB: u32 = 0;
const DIB_RGB_COLORS: u32 = 0;
const SRCCOPY: u32 = 0x00CC_0020;
/// Stretch mode that copies whole pixels (nearest neighbor): crisp pixel art.
const COLORONCOLOR: i32 = 3;

#[repr(C)]
struct WindowClass {
    style: u32,
    window_procedure: extern "system" fn(Handle, u32, usize, isize) -> isize,
    class_extra_bytes: i32,
    window_extra_bytes: i32,
    instance: Handle,
    icon: Handle,
    cursor: Handle,
    background_brush: Handle,
    menu_name: *const u16,
    class_name: *const u16,
}

#[repr(C)]
struct Message {
    window: Handle,
    message: u32,
    w_param: usize,
    l_param: isize,
    time: u32,
    cursor_x: i32,
    cursor_y: i32,
    private: u32,
}

#[repr(C)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct BitmapInfoHeader {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    size_image: u32,
    x_pixels_per_meter: i32,
    y_pixels_per_meter: i32,
    colors_used: u32,
    colors_important: u32,
}

#[repr(C)]
struct BitmapInfo {
    header: BitmapInfoHeader,
    colors: [u32; 1],
}

#[link(name = "user32")]
unsafe extern "system" {
    fn SetProcessDPIAware() -> i32;
    fn RegisterClassW(class: *const WindowClass) -> u16;
    fn AdjustWindowRect(rect: *mut Rect, style: u32, has_menu: i32) -> i32;
    fn CreateWindowExW(
        extended_style: u32,
        class_name: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Handle,
        menu: Handle,
        instance: Handle,
        param: *mut c_void,
    ) -> Handle;
    fn DefWindowProcW(window: Handle, message: u32, w_param: usize, l_param: isize) -> isize;
    fn PeekMessageW(
        message: *mut Message,
        window: Handle,
        filter_min: u32,
        filter_max: u32,
        remove: u32,
    ) -> i32;
    fn TranslateMessage(message: *const Message) -> i32;
    fn DispatchMessageW(message: *const Message) -> isize;
    fn PostQuitMessage(exit_code: i32);
    fn LoadCursorW(instance: Handle, name: *const u16) -> Handle;
    fn GetDC(window: Handle) -> Handle;
    fn SetWindowTextW(window: Handle, text: *const u16) -> i32;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn SetStretchBltMode(device_context: Handle, mode: i32) -> i32;
    fn StretchDIBits(
        device_context: Handle,
        dest_x: i32,
        dest_y: i32,
        dest_width: i32,
        dest_height: i32,
        src_x: i32,
        src_y: i32,
        src_width: i32,
        src_height: i32,
        bits: *const c_void,
        bitmap_info: *const BitmapInfo,
        usage: u32,
        raster_operation: u32,
    ) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(module_name: *const u16) -> Handle;
}

/// Windows calls this function for messages sent directly to our window.
/// We only handle WM_DESTROY (the window was closed): we post WM_QUIT so our
/// event loop knows to stop. Everything else gets the default behavior.
extern "system" fn window_procedure(
    window: Handle,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    if message == WM_DESTROY {
        unsafe { PostQuitMessage(0) };
        return 0;
    }
    unsafe { DefWindowProcW(window, message, w_param, l_param) }
}

/// Windows "W" functions take UTF-16 text ending in a 0 character.
fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

pub struct Window {
    handle: Handle,
    device_context: Handle,
    client_width: i32,
    client_height: i32,
    /// Keys that went down during the latest `process_events` call.
    keys_pressed: [bool; 256],
    /// Reused buffer of packed 0x00RRGGBB pixels handed to Windows.
    display_pixels: Vec<u32>,
}

impl Window {
    /// Opens a window whose drawable area (the "client area", excluding the
    /// title bar and borders) is exactly `client_width` × `client_height`.
    pub fn open(title: &str, client_width: i32, client_height: i32) -> Result<Window, String> {
        let class_name = to_wide("MikageShrineWindow");
        let title = to_wide(title);

        unsafe {
            // Without this, Windows would blur-scale our window on high-DPI screens.
            SetProcessDPIAware();

            let instance = GetModuleHandleW(null());
            let window_class = WindowClass {
                style: CS_OWNDC,
                window_procedure,
                class_extra_bytes: 0,
                window_extra_bytes: 0,
                instance,
                icon: null_mut(),
                cursor: LoadCursorW(null_mut(), IDC_ARROW as *const u16),
                background_brush: null_mut(),
                menu_name: null(),
                class_name: class_name.as_ptr(),
            };
            if RegisterClassW(&window_class) == 0 {
                return Err("RegisterClassW failed".to_string());
            }

            // Grow the outer size so the inner client area is the size we want.
            let style = WS_FIXED_SIZE_WINDOW | WS_VISIBLE;
            let mut outer = Rect { left: 0, top: 0, right: client_width, bottom: client_height };
            AdjustWindowRect(&mut outer, style, 0);

            let handle = CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                style,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                outer.right - outer.left,
                outer.bottom - outer.top,
                null_mut(),
                null_mut(),
                instance,
                null_mut(),
            );
            if handle.is_null() {
                return Err("CreateWindowExW failed".to_string());
            }

            // CS_OWNDC means this device context stays valid for the window's lifetime.
            let device_context = GetDC(handle);
            SetStretchBltMode(device_context, COLORONCOLOR);

            Ok(Window {
                handle,
                device_context,
                client_width,
                client_height,
                keys_pressed: [false; 256],
                display_pixels: Vec::new(),
            })
        }
    }

    /// Handles every message Windows has queued for us since last frame.
    /// Returns false once the window has been closed.
    pub fn process_events(&mut self) -> bool {
        self.keys_pressed = [false; 256];
        let mut message: Message = unsafe { std::mem::zeroed() };

        while unsafe { PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) } != 0 {
            match message.message {
                WM_QUIT => return false,
                WM_KEYDOWN | WM_SYSKEYDOWN => {
                    // Bit 30 of l_param is set when this is an auto-repeat
                    // (key held down); we only want the first press.
                    let is_repeat = (message.l_param >> 30) & 1 == 1;
                    if !is_repeat {
                        self.keys_pressed[message.w_param & 0xFF] = true;
                    }
                }
                _ => {}
            }
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        true
    }

    /// True if `key` (a `key::` constant) went down since the previous frame.
    pub fn was_key_pressed(&self, key: usize) -> bool {
        self.keys_pressed[key]
    }

    pub fn set_title(&self, title: &str) {
        let title = to_wide(title);
        unsafe { SetWindowTextW(self.handle, title.as_ptr()) };
    }

    /// Copies the framebuffer to the window, stretching it to fill the client
    /// area. With COLORONCOLOR each framebuffer pixel becomes a solid block.
    pub fn present(&mut self, framebuffer: &Framebuffer) {
        framebuffer.to_display_pixels(&mut self.display_pixels);

        let bitmap_info = BitmapInfo {
            header: BitmapInfoHeader {
                size: std::mem::size_of::<BitmapInfoHeader>() as u32,
                width: framebuffer.width as i32,
                // Negative height = rows are stored top-down, like our framebuffer.
                height: -(framebuffer.height as i32),
                planes: 1,
                bit_count: 32,
                compression: BI_RGB,
                size_image: 0,
                x_pixels_per_meter: 0,
                y_pixels_per_meter: 0,
                colors_used: 0,
                colors_important: 0,
            },
            colors: [0],
        };

        unsafe {
            StretchDIBits(
                self.device_context,
                0,
                0,
                self.client_width,
                self.client_height,
                0,
                0,
                framebuffer.width as i32,
                framebuffer.height as i32,
                self.display_pixels.as_ptr() as *const c_void,
                &bitmap_info,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
        }
    }
}
