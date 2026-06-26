use futures_util::{SinkExt, StreamExt};
use tokio::time::{sleep, Duration};
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use std::sync::Arc;

use crate::config::{Config, SecureStorage};
use crate::protocol::{ClientMessage, ServerMessage};
use crate::login_handler::{LoginHandler, LoginCredentials};

pub struct WebSocketClient {
    config: Config,
    secure_storage: Option<SecureStorage>,
    login_in_progress: Arc<Mutex<bool>>,
    outgoing_tx: Option<mpsc::UnboundedSender<String>>,
}

impl WebSocketClient {
    pub fn new(config: Config) -> Self {
        let secure_storage = SecureStorage::load();
        
        if let Some(ref storage) = secure_storage {
            tracing::info!("Loaded permanent key for client: {}", storage.client_id);
        } else {
            tracing::info!("No permanent key found, will authenticate with temp key");
        }

        Self {
            config,
            secure_storage,
            login_in_progress: Arc::new(Mutex::new(false)),
            outgoing_tx: None,
        }
    }

    pub async fn run(&mut self) {
        loop {
            tracing::info!("Connecting to server: {}", self.config.server_url);
            
            match self.connect_and_handle().await {
                Ok(_) => tracing::info!("Connection closed normally"),
                Err(e) => tracing::error!("Connection error: {}", e),
            }

            tracing::info!("Reconnecting in 5 seconds...");
            sleep(Duration::from_secs(5)).await;
        }
    }

    fn handle_auth_error(&mut self) {
        tracing::error!("Authentication failed, attempting to recover...");
        
        // If authentication failed with permanent key, clear it
        if self.secure_storage.is_some() {
            tracing::warn!("Permanent key authentication failed, clearing stored credentials");
            self.secure_storage = None;
            let _ = std::fs::remove_file("client_data.enc");
        }
        
        // Reload config to check for new temp_key
        tracing::info!("Reloading config.json to check for updated authentication key...");
        match Config::load() {
            Ok(new_config) => {
                self.config = new_config;
                tracing::info!("Config reloaded successfully");
            }
            Err(e) => {
                tracing::error!("Failed to reload config: {}", e);
            }
        }
    }

    async fn connect_and_handle(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (ws_stream, _) = connect_async(&self.config.server_url).await?;
        
        tracing::info!("WebSocket connected successfully");

        let (mut write, mut read) = ws_stream.split();

        // Create channel for outgoing messages
        let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<String>();
        self.outgoing_tx = Some(outgoing_tx);

        // Authenticate
        let auth_message = if let Some(ref storage) = self.secure_storage {
            // Use permanent key
            ClientMessage::AuthPermanent {
                permanent_key: storage.permanent_key.clone(),
                client_id: storage.client_id.clone(),
            }
        } else {
            // Use temp key
            match &self.config.temp_key {
                Some(temp_key) => ClientMessage::AuthTemp {
                    temp_key: temp_key.clone(),
                },
                None => {
                    return Err("No authentication key available".into());
                }
            }
        };

        let auth_json = auth_message.to_json()?;
        write.send(Message::Text(auth_json)).await?;
        tracing::info!("Authentication message sent");

        // Handle messages
        loop {
            tokio::select! {
                // Handle incoming messages from server
                message = read.next() => {
                    match message {
                        Some(Ok(Message::Text(text))) => {
                            if let Err(e) = self.handle_message(&text).await {
                                tracing::error!("Error handling message: {}", e);
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            tracing::info!("Server closed connection");
                            break;
                        }
                        Some(Ok(_)) => {}
                        Some(Err(e)) => {
                            tracing::error!("WebSocket error: {}", e);
                            break;
                        }
                        None => break,
                    }
                }
                // Handle outgoing progress messages
                Some(msg_json) = outgoing_rx.recv() => {
                    if let Err(e) = write.send(Message::Text(msg_json)).await {
                        tracing::error!("Failed to send progress message: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_message(
        &mut self,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let server_msg = ServerMessage::from_json(text)?;

        match server_msg {
            ServerMessage::AuthSuccess { permanent_key, client_id } => {
                tracing::info!("Authentication successful! Client ID: {}", client_id);
                
                // Save permanent key
                let storage = SecureStorage::new(permanent_key, client_id);
                storage.save()?;
                self.secure_storage = Some(storage);
                
                tracing::info!("Permanent key saved securely");
            }
            
            ServerMessage::AuthError { message } => {
                tracing::error!("Authentication error: {}", message);
                self.handle_auth_error();
                return Err(message.into());
            }
            
            ServerMessage::Login { email, password, pin, otp } => {
                tracing::info!("Received login request for email: {}", email);
                
                // Check if login is already in progress
                let mut login_lock = self.login_in_progress.lock().await;
                if *login_lock {
                    tracing::warn!("Login already in progress, rejecting new request");
                    let error_msg = ClientMessage::Reporting {
                        message: "Login already in progress, please wait".to_string(),
                    };
                    if let Some(ref tx) = self.outgoing_tx {
                        let _ = tx.send(error_msg.to_json()?);
                    }
                    return Ok(());
                }
                
                // Mark login as in progress
                *login_lock = true;
                drop(login_lock); // Release the lock
                
                // Clone necessary data for the task
                let credentials = LoginCredentials {
                    email: email.clone(),
                    password: password.clone(),
                    pin: pin.clone(),
                    otp: otp.clone(),
                };
                
                let login_in_progress = Arc::clone(&self.login_in_progress);
                let outgoing_tx = self.outgoing_tx.as_ref().unwrap().clone();
                
                // Create channel for progress reporting
                let (tx, mut rx) = mpsc::unbounded_channel::<String>();
                
                // Spawn login task
                tokio::spawn(async move {
                    let result = Self::handle_login_task(credentials, tx).await;
                    
                    // Mark login as complete
                    let mut login_lock = login_in_progress.lock().await;
                    *login_lock = false;
                    
                    result
                });
                
                // Forward progress messages to WebSocket via outgoing channel
                tokio::spawn(async move {
                    while let Some(progress_msg) = rx.recv().await {
                        let report = ClientMessage::Reporting {
                            message: progress_msg,
                        };
                        if let Ok(json) = report.to_json() {
                            let _ = outgoing_tx.send(json);
                        }
                    }
                });
            }
        }

        Ok(())
    }

    async fn handle_login_task(
        credentials: LoginCredentials,
        progress_tx: mpsc::UnboundedSender<String>,
    ) -> Result<(), String> {
        // Get resources path
        let resources_path = std::env::current_dir()
            .map_err(|e| format!("Error obteniendo directorio actual: {}", e))?
            .join("resources")
            .to_string_lossy()
            .to_string();

        // Create login handler (should never fail now since HID connection is checked in execute_login)
        let mut handler = LoginHandler::new(resources_path)
            .expect("Failed to create LoginHandler - this should not happen");

        // Execute login with progress reporting
        let result = handler
            .execute_login(credentials, |msg| {
                let _ = progress_tx.send(msg);
            })
            .await;

        match result {
            Ok(()) => {
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Login error: {}", e);
                let _ = progress_tx.send(error_msg.clone());
                Err(error_msg)
            }
        }
    }
}
