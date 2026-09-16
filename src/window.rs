use std::{ffi::c_void, mem::zeroed, ptr::null};

type Handle = isize;

const CS_OWNDC: u32 = 0x0020;
const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
const CW_USEDEFAULT: i32 = i32::MIN;
const PM_REMOVE: u32 = 0x0001;
const WM_CLOSE: u32 = 0x0010;
const WM_DESTROY: u32 = 0x0002;
const WM_QUIT: u32 = 0x0012;
const WM_KEYDOWN: u32 = 0x0100;
const SW_SHOW: i32 = 5;
const BI_RGB: u32 = 0;
const DIB_RGB_COLORS: u32 = 0;
const SRCCOPY: u32 = 0x00CC_0020;
const IDC_ARROW: *const u16 = 32512usize as *const u16;
const VK_ESCAPE: usize = 0x1B;
const VK_SPACE: usize = 0x20;
const VK_LEFT: usize = 0x25;
const VK_UP: usize = 0x26;
const VK_RIGHT: usize = 0x27;
const VK_DOWN: usize = 0x28;

#[derive(Clone, Copy)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Pause,
}

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct Message {
    window: Handle,
    message: u32,
    w_param: usize,
    l_param: isize,
    time: u32,
    point: Point,
    private: u32,
}

#[repr(C)]
struct WindowClass {
    style: u32,
    procedure: Option<unsafe extern "system" fn(Handle, u32, usize, isize) -> isize>,
    class_extra: i32,
    window_extra: i32,
    instance: Handle,
    icon: Handle,
    cursor: Handle,
    background: Handle,
    menu_name: *const u16,
    class_name: *const u16,
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
    image_size: u32,
    x_pixels_per_meter: i32,
    y_pixels_per_meter: i32,
    colors_used: u32,
    colors_important: u32,
}

#[repr(C)]
struct RgbQuad {
    blue: u8,
    green: u8,
    red: u8,
    reserved: u8,
}

#[repr(C)]
struct BitmapInfo {
    header: BitmapInfoHeader,
    colors: [RgbQuad; 1],
}

#[link(name = "user32")]
extern "system" {
    fn RegisterClassW(class: *const WindowClass) -> u16;
    fn CreateWindowExW(
        extended_style: u32,
        class_name: *const u16,
        window_name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Handle,
        menu: Handle,
        instance: Handle,
        parameter: *mut c_void,
    ) -> Handle;
    fn DefWindowProcW(window: Handle, message: u32, w_param: usize, l_param: isize) -> isize;
    fn DestroyWindow(window: Handle) -> i32;
    fn PostQuitMessage(exit_code: i32);
    fn LoadCursorW(instance: Handle, cursor_name: *const u16) -> Handle;
    fn AdjustWindowRect(rect: *mut Rect, style: u32, has_menu: i32) -> i32;
    fn ShowWindow(window: Handle, command: i32) -> i32;
    fn UpdateWindow(window: Handle) -> i32;
    fn PeekMessageW(
        message: *mut Message,
        window: Handle,
        minimum: u32,
        maximum: u32,
        remove: u32,
    ) -> i32;
    fn TranslateMessage(message: *const Message) -> i32;
    fn DispatchMessageW(message: *const Message) -> isize;
    fn GetDC(window: Handle) -> Handle;
    fn ReleaseDC(window: Handle, device_context: Handle) -> i32;
    fn GetAsyncKeyState(virtual_key: i32) -> i16;
    fn SetWindowTextW(window: Handle, text: *const u16) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(module_name: *const u16) -> Handle;
}

#[link(name = "gdi32")]
extern "system" {
    fn StretchDIBits(
        device_context: Handle,
        destination_x: i32,
        destination_y: i32,
        destination_width: i32,
        destination_height: i32,
        source_x: i32,
        source_y: i32,
        source_width: i32,
        source_height: i32,
        pixels: *const c_void,
        bitmap_info: *const BitmapInfo,
        usage: u32,
        raster_operation: u32,
    ) -> i32;
}

pub struct NativeWindow {
    handle: Handle,
    width: usize,
    height: usize,
}

impl NativeWindow {
    pub fn new(title: &str, width: usize, height: usize) -> Self {
        let class_name = wide("CreativeZoneWindow");
        let title = wide(title);

        unsafe {
            let instance = GetModuleHandleW(null());
            let class = WindowClass {
                style: CS_OWNDC,
                procedure: Some(window_procedure),
                class_extra: 0,
                window_extra: 0,
                instance,
                icon: 0,
                cursor: LoadCursorW(0, IDC_ARROW),
                background: 0,
                menu_name: null(),
                class_name: class_name.as_ptr(),
            };
            RegisterClassW(&class);

            let mut bounds = Rect {
                left: 0,
                top: 0,
                right: width as i32,
                bottom: height as i32,
            };
            AdjustWindowRect(&mut bounds, WS_OVERLAPPEDWINDOW, 0);

            let handle = CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                bounds.right - bounds.left,
                bounds.bottom - bounds.top,
                0,
                0,
                instance,
                std::ptr::null_mut(),
            );
            assert!(handle != 0, "No se pudo abrir la ventana");
            ShowWindow(handle, SW_SHOW);
            UpdateWindow(handle);

            Self {
                handle,
                width,
                height,
            }
        }
    }

    pub fn pump_messages(&self) -> Option<Vec<Key>> {
        let mut pressed = Vec::new();
        unsafe {
            let mut message: Message = zeroed();
            while PeekMessageW(&mut message, 0, 0, 0, PM_REMOVE) != 0 {
                if message.message == WM_QUIT {
                    return None;
                }
                if message.message == WM_KEYDOWN && message.l_param & (1 << 30) == 0 {
                    match message.w_param {
                        VK_ESCAPE => {
                            DestroyWindow(self.handle);
                            return None;
                        }
                        0x41 | VK_LEFT => pressed.push(Key::Left),
                        0x44 | VK_RIGHT => pressed.push(Key::Right),
                        0x57 | VK_UP => pressed.push(Key::Up),
                        0x53 | VK_DOWN => pressed.push(Key::Down),
                        VK_SPACE => pressed.push(Key::Pause),
                        _ => {}
                    }
                }
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        Some(pressed)
    }

    pub fn is_key_down(&self, key: Key) -> bool {
        unsafe {
            match key {
                Key::Left => key_down(0x41) || key_down(VK_LEFT),
                Key::Right => key_down(0x44) || key_down(VK_RIGHT),
                Key::Up => key_down(0x57) || key_down(VK_UP),
                Key::Down => key_down(0x53) || key_down(VK_DOWN),
                Key::Pause => key_down(VK_SPACE),
            }
        }
    }

    pub fn set_title(&self, title: &str) {
        let title = wide(title);
        unsafe {
            SetWindowTextW(self.handle, title.as_ptr());
        }
    }

    pub fn present(&self, pixels: &[u32]) {
        assert_eq!(pixels.len(), self.width * self.height);
        let bitmap_info = BitmapInfo {
            header: BitmapInfoHeader {
                size: std::mem::size_of::<BitmapInfoHeader>() as u32,
                width: self.width as i32,
                height: -(self.height as i32),
                planes: 1,
                bit_count: 32,
                compression: BI_RGB,
                image_size: 0,
                x_pixels_per_meter: 0,
                y_pixels_per_meter: 0,
                colors_used: 0,
                colors_important: 0,
            },
            colors: [RgbQuad {
                blue: 0,
                green: 0,
                red: 0,
                reserved: 0,
            }],
        };

        unsafe {
            let device_context = GetDC(self.handle);
            StretchDIBits(
                device_context,
                0,
                0,
                self.width as i32,
                self.height as i32,
                0,
                0,
                self.width as i32,
                self.height as i32,
                pixels.as_ptr().cast(),
                &bitmap_info,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            ReleaseDC(self.handle, device_context);
        }
    }
}

unsafe extern "system" fn window_procedure(
    window: Handle,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    match message {
        WM_CLOSE => {
            DestroyWindow(window);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(window, message, w_param, l_param),
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

unsafe fn key_down(virtual_key: usize) -> bool {
    GetAsyncKeyState(virtual_key as i32) < 0
}
