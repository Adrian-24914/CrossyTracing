use std::{ffi::c_void, mem::zeroed, ptr::null};

type Handle = isize;

const CS_OWNDC: u32 = 0x0020;
const CS_HREDRAW: u32 = 0x0002;
const CS_VREDRAW: u32 = 0x0001;
const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
const CW_USEDEFAULT: i32 = i32::MIN;
const PM_REMOVE: u32 = 0x0001;
const WM_CLOSE: u32 = 0x0010;
const WM_DESTROY: u32 = 0x0002;
const WM_QUIT: u32 = 0x0012;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_KEYDOWN: u32 = 0x0100;
const SW_SHOW: i32 = 5;
const BI_RGB: u32 = 0;
const DIB_RGB_COLORS: u32 = 0;
const SRCCOPY: u32 = 0x00CC_0020;
const BLACKNESS: u32 = 0x0000_0042;
const HALFTONE: i32 = 4;
const SWP_NOZORDER: u32 = 0x0004;
const SWP_NOACTIVATE: u32 = 0x0010;
const SPI_GETWORKAREA: u32 = 0x0030;
const IDC_ARROW: *const u16 = 32512usize as *const u16;
const VK_ESCAPE: usize = 0x1B;
const VK_SPACE: usize = 0x20;
const VK_LEFT: usize = 0x25;
const VK_UP: usize = 0x26;
const VK_RIGHT: usize = 0x27;
const VK_DOWN: usize = 0x28;
const VK_R: usize = 0x52;
const VK_O: usize = 0x4F;
const VK_F1: usize = 0x70;
const VK_F2: usize = 0x71;
const VK_F3: usize = 0x72;

#[derive(Clone, Copy)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Pause,
    Reset,
    Orthographic,
    DisplayPerformance,
    DisplayHigh,
    DisplayUltra,
    Start,
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
    fn GetClientRect(window: Handle, rect: *mut Rect) -> i32;
    fn SetWindowPos(
        window: Handle,
        insert_after: Handle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> i32;
    fn SetProcessDPIAware() -> i32;
    fn SystemParametersInfoW(action: u32, parameter: u32, value: *mut c_void, update: u32) -> i32;
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
    fn SetStretchBltMode(device_context: Handle, mode: i32) -> i32;
    fn SetBrushOrgEx(device_context: Handle, x: i32, y: i32, previous: *mut Point) -> i32;
    fn PatBlt(
        device_context: Handle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        raster_operation: u32,
    ) -> i32;
}

pub struct NativeWindow {
    handle: Handle,
}

impl NativeWindow {
    pub fn new(title: &str, width: usize, height: usize) -> Self {
        let class_name = wide("CreativeZoneWindow");
        let title = wide(title);

        unsafe {
            // Sin esto, el escalado de Windows puede volver borrosa una imagen
            // que ya fue renderizada a alta resolución.
            SetProcessDPIAware();
            let instance = GetModuleHandleW(null());
            let class = WindowClass {
                style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
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
            let window = Self { handle };
            window.set_client_size(width, height);
            ShowWindow(handle, SW_SHOW);
            UpdateWindow(handle);

            window
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
                        0x57 => pressed.push(Key::Start),
                        VK_UP => pressed.push(Key::Up),
                        0x53 | VK_DOWN => pressed.push(Key::Down),
                        VK_SPACE => pressed.push(Key::Pause),
                        VK_R => pressed.push(Key::Reset),
                        VK_O => pressed.push(Key::Orthographic),
                        VK_F1 => pressed.push(Key::DisplayPerformance),
                        VK_F2 => pressed.push(Key::DisplayHigh),
                        VK_F3 => pressed.push(Key::DisplayUltra),
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
                Key::Reset => key_down(VK_R),
                Key::Orthographic => key_down(VK_O),
                Key::DisplayPerformance => key_down(VK_F1),
                Key::DisplayHigh => key_down(VK_F2),
                Key::DisplayUltra => key_down(VK_F3),
                Key::Start => key_down(0x57),
            }
        }
    }

    pub fn set_title(&self, title: &str) {
        let title = wide(title);
        unsafe {
            SetWindowTextW(self.handle, title.as_ptr());
        }
    }

    pub fn set_client_size(&self, width: usize, height: usize) {
        let mut requested_bounds = Rect {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        };
        unsafe {
            AdjustWindowRect(&mut requested_bounds, WS_OVERLAPPEDWINDOW, 0);
            let non_client_width = requested_bounds.right - requested_bounds.left - width as i32;
            let non_client_height = requested_bounds.bottom - requested_bounds.top - height as i32;

            let mut work_area: Rect = zeroed();
            let has_work_area =
                SystemParametersInfoW(SPI_GETWORKAREA, 0, (&mut work_area as *mut Rect).cast(), 0)
                    != 0;
            if !has_work_area {
                work_area = Rect {
                    left: 0,
                    top: 0,
                    right: requested_bounds.right - requested_bounds.left,
                    bottom: requested_bounds.bottom - requested_bounds.top,
                };
            }

            let work_width = work_area.right - work_area.left;
            let work_height = work_area.bottom - work_area.top;
            let mut client_width = width;
            let mut client_height = height;
            if requested_bounds.right - requested_bounds.left > work_width
                || requested_bounds.bottom - requested_bounds.top > work_height
            {
                let maximum_client_width = (work_width - non_client_width).max(1) as usize;
                let maximum_client_height = (work_height - non_client_height).max(1) as usize;
                (client_width, client_height) =
                    fitted_size(maximum_client_width, maximum_client_height, width, height)
                        .unwrap_or((1, 1));
            }

            let mut bounds = Rect {
                left: 0,
                top: 0,
                right: client_width as i32,
                bottom: client_height as i32,
            };
            AdjustWindowRect(&mut bounds, WS_OVERLAPPEDWINDOW, 0);
            let outer_width = bounds.right - bounds.left;
            let outer_height = bounds.bottom - bounds.top;
            let x = work_area.left + (work_width - outer_width).max(0) / 2;
            let y = work_area.top + (work_height - outer_height).max(0) / 2;
            SetWindowPos(
                self.handle,
                0,
                x,
                y,
                outer_width,
                outer_height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    pub fn client_size(&self) -> (usize, usize) {
        let mut bounds = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        unsafe {
            if GetClientRect(self.handle, &mut bounds) == 0 {
                return (0, 0);
            }
        }
        (
            (bounds.right - bounds.left).max(0) as usize,
            (bounds.bottom - bounds.top).max(0) as usize,
        )
    }

    /// Presenta un buffer de cualquier resolución dentro del área cliente real.
    /// Conserva la proporción del juego y centra barras negras cuando hace falta.
    pub fn present_scaled(&self, pixels: &[u32], source_width: usize, source_height: usize) {
        assert_eq!(pixels.len(), source_width * source_height);
        let (client_width, client_height) = self.client_size();
        let Some(viewport) =
            fitted_viewport(client_width, client_height, source_width, source_height)
        else {
            return;
        };
        let bitmap_info = BitmapInfo {
            header: BitmapInfoHeader {
                size: std::mem::size_of::<BitmapInfoHeader>() as u32,
                width: source_width as i32,
                height: -(source_height as i32),
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
            SetStretchBltMode(device_context, HALFTONE);
            SetBrushOrgEx(device_context, 0, 0, std::ptr::null_mut());
            StretchDIBits(
                device_context,
                viewport.x as i32,
                viewport.y as i32,
                viewport.width as i32,
                viewport.height as i32,
                0,
                0,
                source_width as i32,
                source_height as i32,
                pixels.as_ptr().cast(),
                &bitmap_info,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            paint_letterbox_bars(device_context, client_width, client_height, viewport);
            ReleaseDC(self.handle, device_context);
        }
    }
}

unsafe fn paint_letterbox_bars(
    device_context: Handle,
    client_width: usize,
    client_height: usize,
    viewport: Viewport,
) {
    for (x, y, width, height) in letterbox_bars(client_width, client_height, viewport) {
        if width > 0 && height > 0 {
            PatBlt(
                device_context,
                x as i32,
                y as i32,
                width as i32,
                height as i32,
                BLACKNESS,
            );
        }
    }
}

fn letterbox_bars(
    client_width: usize,
    client_height: usize,
    viewport: Viewport,
) -> [(usize, usize, usize, usize); 4] {
    let right = viewport.x + viewport.width;
    let bottom = viewport.y + viewport.height;
    [
        (0, 0, client_width, viewport.y),
        (0, bottom, client_width, client_height - bottom),
        (0, viewport.y, viewport.x, viewport.height),
        (right, viewport.y, client_width - right, viewport.height),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Viewport {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

fn fitted_viewport(
    client_width: usize,
    client_height: usize,
    source_width: usize,
    source_height: usize,
) -> Option<Viewport> {
    if client_width == 0 || client_height == 0 || source_width == 0 || source_height == 0 {
        return None;
    }

    let (width, height) = fitted_size(client_width, client_height, source_width, source_height)?;

    Some(Viewport {
        x: (client_width - width) / 2,
        y: (client_height - height) / 2,
        width,
        height,
    })
}

fn fitted_size(
    available_width: usize,
    available_height: usize,
    source_width: usize,
    source_height: usize,
) -> Option<(usize, usize)> {
    if available_width == 0 || available_height == 0 || source_width == 0 || source_height == 0 {
        return None;
    }

    let available_is_wider = available_width as u64 * source_height as u64
        > available_height as u64 * source_width as u64;
    if available_is_wider {
        let width = available_height as u64 * source_width as u64 / source_height as u64;
        Some((width as usize, available_height))
    } else {
        let height = available_width as u64 * source_height as u64 / source_width as u64;
        Some((available_width, height as usize))
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
        // El framebuffer ya cubre el viewport y las barras. Evitar que Windows
        // borre primero el fondo elimina el destello negro durante un resize.
        WM_ERASEBKGND => 1,
        _ => DefWindowProcW(window, message, w_param, l_param),
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

unsafe fn key_down(virtual_key: usize) -> bool {
    GetAsyncKeyState(virtual_key as i32) < 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitted_viewport_adds_side_bars_to_a_wide_window() {
        assert_eq!(
            fitted_viewport(1920, 1080, 800, 600),
            Some(Viewport {
                x: 240,
                y: 0,
                width: 1440,
                height: 1080,
            })
        );
    }

    #[test]
    fn fitted_viewport_adds_top_and_bottom_bars_to_a_tall_window() {
        assert_eq!(
            fitted_viewport(800, 800, 800, 600),
            Some(Viewport {
                x: 0,
                y: 100,
                width: 800,
                height: 600,
            })
        );
    }

    #[test]
    fn fitted_viewport_ignores_a_minimized_window() {
        assert_eq!(fitted_viewport(0, 0, 800, 600), None);
    }

    #[test]
    fn matching_aspect_ratio_never_clears_over_the_game_image() {
        let viewport = fitted_viewport(960, 720, 720, 540).unwrap();
        let bars = letterbox_bars(960, 720, viewport);

        assert!(bars
            .into_iter()
            .all(|(_, _, width, height)| width == 0 || height == 0));
    }

    #[test]
    fn wide_window_only_clears_the_two_side_bars() {
        let viewport = fitted_viewport(1280, 720, 720, 540).unwrap();
        let bars = letterbox_bars(1280, 720, viewport);
        let visible_bars: Vec<_> = bars
            .into_iter()
            .filter(|(_, _, width, height)| *width > 0 && *height > 0)
            .collect();

        assert_eq!(visible_bars, vec![(0, 0, 160, 720), (1120, 0, 160, 720)]);
    }

    #[test]
    fn fitted_size_limits_a_preset_without_changing_its_proportions() {
        assert_eq!(fitted_size(1200, 700, 1280, 960), Some((933, 700)));
    }
}
