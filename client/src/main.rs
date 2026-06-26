#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod config;
mod protocol;
mod websocket_client;
mod tray_app;
mod automation;
mod login_handler;
mod hid_config;
mod hid_control;

use tracing_subscriber;

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging to file
    let log_file_path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("rolatam_client.log")))
        .unwrap_or_else(|| std::path::PathBuf::from("rolatam_client.log"));
    
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)  // Truncate (clear) on each start
        .open(&log_file_path)?;
    
    tracing_subscriber::fmt()
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::sync::Mutex::new(log_file))
        .with_ansi(false)
        .init();
    
    // Set panic hook to log panics
    std::panic::set_hook(Box::new(|panic_info| {
        tracing::error!("PANIC: {:?}", panic_info);
    }));
    
    tracing::info!("========================================");
    tracing::info!("ROLatam Account Manager started");
    tracing::info!("Log file: {:?}", log_file_path);
    tracing::info!("========================================");

    // Check command line arguments
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() > 1 && args[1] == "--standalone" {
        // Run in standalone mode (no tray, just console)
        run_standalone()?;
    } else {
        // Default: run with system tray
        run_tray()?;
    }

    Ok(())
}

#[cfg(windows)]
fn run_tray() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting in system tray mode");
    
    // Validate resources folder exists
    if let Err(e) = validate_resources() {
        tracing::error!("Resource validation failed: {}", e);
        return Err(e);
    }
    
    let runtime = tokio::runtime::Runtime::new()?;
    tracing::info!("Tokio runtime created");
    
    runtime.block_on(async {
        tracing::info!("Loading config...");
        let config = match config::Config::load() {
            Ok(c) => {
                tracing::info!("Config loaded successfully");
                c
            }
            Err(e) => {
                tracing::error!("Failed to load config: {}", e);
                return Err(e);
            }
        };
        
        tracing::info!("Starting TrayApp...");
        match tray_app::TrayApp::run_with_websocket(config).await {
            Ok(_) => {
                tracing::info!("TrayApp exited normally");
                Ok(())
            }
            Err(e) => {
                tracing::error!("TrayApp error: {}", e);
                Err(e)
            }
        }
    })?;

    Ok(())
}

#[cfg(windows)]
fn validate_resources() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;
    
    let resources_dir = Path::new("resources");
    
    if !resources_dir.exists() {
        return Err("resources/ folder not found. Please ensure it exists next to the executable".into());
    }
    
    tracing::info!("Resources folder found");
    
    // List of required files (icon.ico NOT needed - it's embedded in exe)
    let required_files = vec![
        "login_page.png",
        "otp_page.png",
        "server_select.png",
        "pin_select.png",
        "ok_pin.png",
        "0.bmp", "1.bmp", "2.bmp", "3.bmp", "4.bmp",
        "5.bmp", "6.bmp", "7.bmp", "8.bmp", "9.bmp",
    ];
    
    let mut missing_files = Vec::new();
    
    for file in &required_files {
        let file_path = resources_dir.join(file);
        if !file_path.exists() {
            missing_files.push(file.to_string());
        }
    }
    
    if !missing_files.is_empty() {
        let error_msg = format!(
            "Missing required resource files: {}",
            missing_files.join(", ")
        );
        tracing::error!("{}", error_msg);
        return Err(error_msg.into());
    }
    
    tracing::info!("All {} required resource files validated successfully", required_files.len());
    Ok(())
}

#[cfg(windows)]
fn run_standalone() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Running in standalone mode");
    
    let runtime = tokio::runtime::Runtime::new()?;
    
    runtime.block_on(async {
        let config = config::Config::load()?;
        let mut client = websocket_client::WebSocketClient::new(config);
        client.run().await;
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;

    Ok(())
}

#[cfg(not(windows))]
fn main() {
    eprintln!("This application is designed to run on Windows only.");
}
