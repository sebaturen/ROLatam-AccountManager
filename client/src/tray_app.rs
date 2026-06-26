use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder, TrayIcon,
};

use crate::config::Config;
use crate::websocket_client::WebSocketClient;

const APP_NAME: &str = "ROLatamClient";

pub struct TrayApp {
    _tray_icon: TrayIcon,
    quit_item_id: tray_icon::menu::MenuId,
}

impl TrayApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Register for Windows startup on first run
        register_startup()?;
        
        // Create menu items
        let menu = Menu::new();
        let status_item = MenuItem::new("Status: Initializing...", false, None);
        let separator = MenuItem::new("─────────────────", false, None);
        let quit_item = MenuItem::new("Exit", true, None);
        
        let quit_item_id = quit_item.id().clone();

        menu.append(&status_item)?;
        menu.append(&separator)?;
        menu.append(&quit_item)?;

        // Create tray icon
        let icon = load_icon()?;
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("ROLatam Account Manager")
            .with_icon(icon)
            .build()?;

        tracing::info!("System tray icon created");

        Ok(Self {
            _tray_icon: tray_icon,
            quit_item_id,
        })
    }

    pub async fn run_with_websocket(config: Config) -> Result<(), Box<dyn std::error::Error>> {
        // Create tray icon first (must be on main thread)
        let tray = Self::new()?;
        let quit_id = tray.quit_item_id.clone();

        // Spawn WebSocket client in background task
        let ws_handle = tokio::spawn(async move {
            let mut client = WebSocketClient::new(config);
            client.run().await;
        });

        // Handle menu events on main thread
        let menu_channel = MenuEvent::receiver();
        
        tracing::info!("System tray running, WebSocket client started");
        
        // Need to pump Windows messages for tray menu to work
        use windows::Win32::UI::WindowsAndMessaging::{
            PeekMessageW, TranslateMessage, DispatchMessageW, MSG, PM_REMOVE,
        };
        
        loop {
            // Process Windows messages (required for tray menu)
            unsafe {
                let mut msg = MSG::default();
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            
            // Check for menu events
            if let Ok(event) = menu_channel.try_recv() {
                if event.id == quit_id {
                    tracing::info!("Exit requested from tray menu");
                    std::process::exit(0);
                }
            }
            
            // Check if websocket died
            if ws_handle.is_finished() {
                tracing::error!("WebSocket client task ended unexpectedly");
                break;
            }
            
            // Small sleep to avoid busy loop
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        Ok(())
    }
}

/// Load icon from embedded resource
fn load_icon() -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
    // Include icon bytes at compile time
    let icon_bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/icon.ico"));
    
    tracing::info!("Loading tray icon from embedded resource");
    
    // Decode ICO to RGBA using image crate
    let img = image::load_from_memory(icon_bytes)?;
    let img = img.to_rgba8();
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();
    
    Ok(tray_icon::Icon::from_rgba(rgba, width, height)?)
}

/// Register application to start with Windows
fn register_startup() -> Result<(), Box<dyn std::error::Error>> {
    use windows::Win32::System::Registry::{
        RegOpenKeyExW, RegSetValueExW, RegQueryValueExW, RegCloseKey,
        HKEY_CURRENT_USER, KEY_WRITE, KEY_READ, REG_SZ,
    };
    use windows::core::PCWSTR;
    use std::os::windows::ffi::OsStrExt;
    
    let subkey = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    let value_name = APP_NAME;
    
    // Get current exe path
    let exe_path = std::env::current_exe()?;
    let exe_path_str = exe_path.to_string_lossy().to_string();
    
    // Convert to wide strings
    let subkey_wide: Vec<u16> = std::ffi::OsStr::new(subkey)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    
    let value_name_wide: Vec<u16> = std::ffi::OsStr::new(value_name)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    
    let exe_path_wide: Vec<u16> = std::ffi::OsStr::new(&exe_path_str)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    
    unsafe {
        let mut hkey = Default::default();
        
        // Open registry key
        let open_result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey_wide.as_ptr()),
            0,
            KEY_READ | KEY_WRITE,
            &mut hkey,
        );
        
        if open_result.is_err() {
            tracing::error!("Failed to open registry key: {:?}", open_result);
            return Err(format!("Registry open failed: {:?}", open_result).into());
        }
        
        // Check if already registered
        let mut buffer = vec![0u16; 512];
        let mut buffer_size = (buffer.len() * 2) as u32;
        
        let query_result = RegQueryValueExW(
            hkey,
            PCWSTR(value_name_wide.as_ptr()),
            None,
            None,
            Some(buffer.as_mut_ptr() as *mut u8),
            Some(&mut buffer_size),
        );
        
        // If not registered or path changed, register it
        if query_result.is_err() || 
           String::from_utf16_lossy(&buffer[..((buffer_size / 2) as usize - 1)]) != exe_path_str {
            
            let set_result = RegSetValueExW(
                hkey,
                PCWSTR(value_name_wide.as_ptr()),
                0,
                REG_SZ,
                Some(&std::slice::from_raw_parts(
                    exe_path_wide.as_ptr() as *const u8,
                    exe_path_wide.len() * 2,
                )),
            );
            
            if set_result.is_err() {
                let _ = RegCloseKey(hkey);
                return Err(format!("Registry write failed: {:?}", set_result).into());
            }
            
            tracing::info!("Registered for Windows startup: {}", exe_path_str);
        } else {
            tracing::info!("Already registered for Windows startup");
        }
        
        let _ = RegCloseKey(hkey);
    }
    
    Ok(())
}
