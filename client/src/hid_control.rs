// HID Control - Serial communication with Pico USB HID device
// Handles mouse and keyboard input via external hardware

use serialport::{SerialPort, SerialPortType};
use std::time::Duration;
use std::sync::Mutex;
use tracing::debug;

use crate::hid_config::*;

/// Singleton HID controller for USB serial communication
pub struct HIDControl {
    port: Mutex<Option<Box<dyn SerialPort>>>,
}

impl HIDControl {
    /// Create a new HID controller instance
    pub fn new() -> Self {
        Self {
            port: Mutex::new(None),
        }
    }

    /// Detect Pico device by description
    fn detect_pico() -> Option<String> {
        let ports = serialport::available_ports().ok()?;
        
        tracing::info!("Scanning for HID device... Found {} serial ports", ports.len());
        
        for port in ports {
            match &port.port_type {
                SerialPortType::UsbPort(info) => {
                    tracing::info!("USB Port: {} | Product: {:?} | Manufacturer: {:?}", 
                           port.port_name, info.product, info.manufacturer);
                    
                    // Check for Pico-specific identifiers
                    if let Some(product) = &info.product {
                        if product.contains("Pico") || product.contains("USB Serial") {
                            tracing::info!("FOUND: Pico device at: {}", port.port_name);
                            return Some(port.port_name);
                        }
                    }
                    
                    if let Some(manufacturer) = &info.manufacturer {
                        if manufacturer.contains("Raspberry Pi") || manufacturer.contains("Pico") {
                            tracing::info!("FOUND: Pico device at: {}", port.port_name);
                            return Some(port.port_name);
                        }
                    }
                }
                _ => {
                    tracing::info!("Non-USB port: {}", port.port_name);
                }
            }
        }
        
        tracing::warn!("No Pico device found in available ports");
        None
    }

    /// Connect to the Pico HID device
    pub fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut port_lock = self.port.lock().unwrap();
        
        // Already connected
        if port_lock.is_some() {
            return Ok(());
        }

        let port_name = Self::detect_pico()
            .ok_or("Please connect HID device")?;

        let port = serialport::new(&port_name, 115200)
            .timeout(Duration::from_secs(1))
            .open()?;

        debug!("Connected to Pico HID at {}", port_name);
        
        // Small delay for device initialization
        std::thread::sleep(Duration::from_millis(50));
        
        *port_lock = Some(port);
        Ok(())
    }

    /// Check if connected to HID device
    pub fn is_connected(&self) -> bool {
        self.port.lock().unwrap().is_some()
    }

    /// Send raw command to HID device
    fn send_command(&self, command: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut port_lock = self.port.lock().unwrap();
        
        let port = port_lock.as_mut()
            .ok_or("Not connected to HID device")?;

        port.write_all(command.as_bytes())?;
        debug!("Sent HID command: {}", command.trim());
        Ok(())
    }

    /// Disconnect from HID device
    pub fn disconnect(&self) {
        let mut port_lock = self.port.lock().unwrap();
        *port_lock = None;
        debug!("Disconnected from HID device");
    }

    /// Get current mouse cursor position (uses Windows API)
    #[cfg(windows)]
    fn get_current_position() -> (i32, i32) {
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        use windows::Win32::Foundation::POINT;
        
        unsafe {
            let mut point = POINT { x: 0, y: 0 };
            let _ = GetCursorPos(&mut point);
            (point.x, point.y)
        }
    }

    #[cfg(not(windows))]
    fn get_current_position() -> (i32, i32) {
        (0, 0)
    }

    /// Move mouse to absolute position
    /// Command format: "M,{dx},{dy}\n"
    pub fn move_to_absolute(&self, target_x: i32, target_y: i32) -> Result<(), Box<dyn std::error::Error>> {
        let (current_x, current_y) = Self::get_current_position();
        
        let dx = target_x - current_x;
        let dy = target_y - current_y;
        
        if dx == 0 && dy == 0 {
            return Ok(());
        }
        
        let command = format!("M,{},{}\n", dx, dy);
        self.send_command(&command)?;
        
        // No delay - instant teleport
        Ok(())
    }

    /// Click mouse button at current position
    /// Command format: "C,{button}\n"
    pub fn click(&self, button: u8) -> Result<(), Box<dyn std::error::Error>> {
        let command = format!("C,{}\n", button);
        self.send_command(&command)?;
        std::thread::sleep(Duration::from_millis(2));
        Ok(())
    }

    /// Move mouse and click at position
    pub fn click_at(&self, x: i32, y: i32, button: u8) -> Result<(), Box<dyn std::error::Error>> {
        self.move_to_absolute(x, y)?;
        std::thread::sleep(Duration::from_millis(20));
        self.click(button)?;
        Ok(())
    }

    /// Press a single key
    /// Command format: "K,{keycode}\n"
    pub fn press_key(&self, keycode: u8) -> Result<(), Box<dyn std::error::Error>> {
        let command = format!("K,{}\n", keycode);
        self.send_command(&command)?;
        // Minimal delay for serial reliability
        std::thread::sleep(Duration::from_millis(2));
        Ok(())
    }

    /// Press key with modifier (e.g., Shift + key)
    /// Command format: "Z,{modifier},{keycode}\n"
    pub fn press_key_with_modifier(&self, modifier: u8, keycode: u8) -> Result<(), Box<dyn std::error::Error>> {
        let command = format!("Z,{},{}\n", modifier, keycode);
        self.send_command(&command)?;
        // Minimal delay for serial reliability
        std::thread::sleep(Duration::from_millis(2));
        Ok(())
    }

    /// Copy text to clipboard and paste using Ctrl+V
    #[cfg(windows)]
    pub fn paste_text(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        use clipboard_win::{formats, set_clipboard};
        
        // Copy to clipboard using clipboard-win
        set_clipboard(formats::Unicode, text)
            .map_err(|e| format!("Failed to set clipboard: {:?}", e))?;
        
        debug!("Copied to clipboard: {} chars", text.len());

        // Small delay to ensure clipboard is ready
        std::thread::sleep(Duration::from_millis(10));

        // Press Ctrl+V with HID (hardware external)
        self.press_key_with_modifier(KEYBOARD_LEFT_CTRL, KEYBOARD_V)?;
        
        debug!("Pasted text via Ctrl+V (hardware)");
        Ok(())
    }

    #[cfg(not(windows))]
    pub fn paste_text(&self, _text: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Non-Windows not supported (HID requires external hardware)
        Err("Paste not supported on non-Windows platforms".into())
    }

    /// Press Enter key
    pub fn press_enter(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.press_key(KEYBOARD_ENTER)
    }

    /// Press Tab key
    pub fn press_tab(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.press_key(KEYBOARD_TAB)
    }
}

impl Drop for HIDControl {
    fn drop(&mut self) {
        self.disconnect();
    }
}
