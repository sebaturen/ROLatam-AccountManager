use opencv::{
    core::{self, Mat, Point, CV_8UC3},
    imgcodecs, imgproc, prelude::*,
};

#[cfg(windows)]
use windows::{
    Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, GetWindowRect,
    },
    Win32::Foundation::{HWND, RECT},
    Win32::Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
        GetDC, ReleaseDC, SelectObject, BitBlt, GetDIBits, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
    },
};

use crate::hid_control::HIDControl;
use crate::hid_config::*;

pub struct AutomationEngine {
    pub hid: HIDControl,
    window_offset_x: i32,
    window_offset_y: i32,
}

#[derive(Debug, Clone)]
pub struct ImageMatch {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl AutomationEngine {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let hid = HIDControl::new();
        // Don't connect here - let login handler manage connection
        // This allows the application to start even if HID is not connected yet

        Ok(Self {
            hid,
            window_offset_x: 0,
            window_offset_y: 0,
        })
    }

    /// Check if ragexe.exe window is in focus
    #[cfg(windows)]
    pub fn is_ragexe_focused(&self) -> bool {
        unsafe {
            let foreground_window = GetForegroundWindow();
            if foreground_window.0.is_null() {
                return false;
            }

            let mut window_title: [u16; 512] = [0; 512];
            let len = GetWindowTextW(foreground_window, &mut window_title);
            
            if len == 0 {
                return false;
            }

            let title = String::from_utf16_lossy(&window_title[..len as usize]);
            title.to_lowercase().contains("ragnarok") || title.to_lowercase().contains("ragexe")
        }
    }

    #[cfg(not(windows))]
    pub fn is_ragexe_focused(&self) -> bool {
        false
    }

    /// Move mouse to corner to avoid interference with capture
    pub fn move_mouse_away(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(windows)]
        {
            // Try to get Ragnarok window bounds
            unsafe {
                let hwnd = GetForegroundWindow();
                if !hwnd.0.is_null() {
                    let mut rect = RECT::default();
                    if GetWindowRect(hwnd, &mut rect).is_ok() {
                        // Move to bottom-right corner of the window (inside the window)
                        let target_x = rect.right - 10;
                        let target_y = rect.bottom - 10;
                        self.hid.move_to_absolute(target_x, target_y)?;
                        return Ok(());
                    }
                }
            }
        }
        
        // Fallback: move to screen top-left if window not found
        self.hid.move_to_absolute(5, 5)?;
        Ok(())
    }

    /// Capture the current screen using Windows GDI and OpenCV (only RO window)
    #[cfg(windows)]
    pub fn capture_screen(&mut self) -> Result<Mat, Box<dyn std::error::Error>> {
        let start = std::time::Instant::now();
        
        unsafe {
            // Get the focused window (should be RO)
            let hwnd = GetForegroundWindow();
            
            // Get window bounds (screen coordinates)
            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);
            
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            
            // Store window offset (images are found at window coords, need to add offset for clicks)
            self.window_offset_x = rect.left;
            self.window_offset_y = rect.top;
            
            // Capture from screen DC at window position
            let hwnd_desktop = HWND(std::ptr::null_mut());
            let hdc_screen = GetDC(hwnd_desktop);
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            
            let hbitmap = CreateCompatibleBitmap(hdc_screen, width, height);
            let _old_obj = SelectObject(hdc_mem, hbitmap);
            
            let _ = BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, rect.left, rect.top, SRCCOPY);
            
            let mut bi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 24,
                    biCompression: BI_RGB.0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [Default::default(); 1],
            };
            
            let buffer_size = ((width * 3 + 3) & !3) * height;
            let mut buffer: Vec<u8> = vec![0; buffer_size as usize];
            
            let _ = GetDIBits(
                hdc_mem,
                hbitmap,
                0,
                height as u32,
                Some(buffer.as_mut_ptr() as *mut _),
                &mut bi,
                DIB_RGB_COLORS,
            );
            
            let _ = DeleteObject(hbitmap);
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(hwnd_desktop, hdc_screen);
            
            let mat = Mat::new_rows_cols_with_data_unsafe(
                height,
                width,
                CV_8UC3,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                opencv::core::Mat_AUTO_STEP,
            )?;
            
            let result = mat.try_clone()?;
            
            let elapsed = start.elapsed();
            tracing::debug!("Screen capture: {:?}", elapsed);
            
            Ok(result)
        }
    }

    #[cfg(not(windows))]
    pub fn capture_screen(&mut self) -> Result<Mat, Box<dyn std::error::Error>> {
        Err("Screen capture only supported on Windows".into())
    }

    /// Find template image on screen using OpenCV
    pub fn find_image_on_screen(
        &mut self,
        template_path: &str,
        threshold: f64,
    ) -> Result<Option<ImageMatch>, Box<dyn std::error::Error>> {
        let start = std::time::Instant::now();
        
        let template = imgcodecs::imread(template_path, imgcodecs::IMREAD_COLOR)?;
        if template.empty() {
            return Err(format!("Failed to load template: {}", template_path).into());
        }

        let screen = self.capture_screen()?;

        let mut result = Mat::default();
        imgproc::match_template(
            &screen,
            &template,
            &mut result,
            imgproc::TM_CCOEFF_NORMED,
            &core::no_array(),
        )?;

        let mut min_val = 0.0;
        let mut max_val = 0.0;
        let mut min_loc = Point::default();
        let mut max_loc = Point::default();

        core::min_max_loc(
            &result,
            Some(&mut min_val),
            Some(&mut max_val),
            Some(&mut min_loc),
            Some(&mut max_loc),
            &core::no_array(),
        )?;

        tracing::debug!("Image search: {:?} ms ({})", start.elapsed().as_millis(), template_path);

        if max_val >= threshold {
            Ok(Some(ImageMatch {
                x: max_loc.x,
                y: max_loc.y,
                width: template.cols(),
                height: template.rows(),
            }))
        } else {
            tracing::debug!("Image NOT found: {} (best confidence: {:.3}, required: {:.3})", template_path, max_val, threshold);
            Ok(None)
        }
    }

    /// Click at specific coordinates using HID (converts window coords to screen coords)
    pub fn click_at(&mut self, x: i32, y: i32) -> Result<(), Box<dyn std::error::Error>> {
        // Convert window-relative coordinates to screen-absolute coordinates
        let screen_x = x + self.window_offset_x;
        let screen_y = y + self.window_offset_y;
        self.hid.click_at(screen_x, screen_y, MOUSE_LEFT_BUTTON)
    }

    /// Paste text using clipboard (much faster)
    pub fn paste_text(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.hid.paste_text(text)
    }

    /// Press Enter using HID
    pub fn press_enter(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.hid.press_enter()
    }

    /// Press Tab using HID
    pub fn press_tab(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.hid.press_tab()
    }
}
