use crate::automation::{AutomationEngine, ImageMatch};
use tokio::time::{sleep, Duration, timeout};

pub struct LoginCredentials {
    pub email: String,
    pub password: String,
    pub pin: String,
    pub otp: String,
}

#[derive(Debug)]
pub enum LoginError {
    RagexeFocusLost,
    Timeout(String),
    AutomationError(String),
}

impl std::fmt::Display for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoginError::RagexeFocusLost => write!(f, "Ragnarok window lost focus"),
            LoginError::Timeout(stage) => write!(f, "Timeout waiting for: {}", stage),
            LoginError::AutomationError(msg) => write!(f, "Automation error: {}", msg),
        }
    }
}

impl std::error::Error for LoginError {}

pub struct LoginHandler {
    automation: AutomationEngine,
    resources_path: String,
}

impl LoginHandler {
    pub fn new(resources_path: String) -> Result<Self, Box<dyn std::error::Error>> {
        let automation = AutomationEngine::new()?;
        Ok(Self {
            automation,
            resources_path,
        })
    }

    /// Execute the complete login flow
    pub async fn execute_login<F>(
        &mut self,
        credentials: LoginCredentials,
        mut report_progress: F,
    ) -> Result<(), LoginError>
    where
        F: FnMut(String),
    {
        report_progress("Starting login process...".to_string());

        // Step 0: Wait for HID device to be connected
        self.wait_for_hid_connection(&mut report_progress).await?;

        // Step 1: Wait for ragexe.exe to be focused
        self.wait_for_ragexe_focus(&mut report_progress).await?;

        // Step 2: Find login page and enter credentials
        self.enter_login_credentials(&credentials, &mut report_progress).await?;

        // Step 3: Handle OTP
        self.enter_otp(&credentials.otp, &mut report_progress).await?;

        // Step 4: Select server
        self.select_server(&mut report_progress).await?;

        // Step 5: Enter PIN
        self.enter_pin(&credentials.pin, &mut report_progress).await?;

        report_progress("Login completed successfully".to_string());
        Ok(())
    }

    /// Wait for HID device to be connected
    async fn wait_for_hid_connection<F>(&self, report_progress: &mut F) -> Result<(), LoginError>
    where
        F: FnMut(String),
    {
        // Check if already connected
        if self.automation.hid.is_connected() {
            report_progress("HID device ready".to_string());
            return Ok(());
        }

        report_progress("Waiting for HID device connection...".to_string());
        report_progress("Please connect HID device".to_string());

        let result = timeout(Duration::from_secs(60), async {
            loop {
                // Try to connect
                if let Ok(()) = self.automation.hid.connect() {
                    return Ok::<(), LoginError>(());
                }
                sleep(Duration::from_secs(2)).await;
            }
        })
        .await;

        match result {
            Ok(Ok(())) => {
                report_progress("HID device connected successfully".to_string());
                Ok(())
            }
            _ => {
                Err(LoginError::Timeout("HID device connection".to_string()))
            }
        }
    }

    /// Wait for ragexe.exe to be focused
    async fn wait_for_ragexe_focus<F>(&self, report_progress: &mut F) -> Result<(), LoginError>
    where
        F: FnMut(String),
    {
        report_progress("Waiting for ragexe.exe to be focused...".to_string());

        let result = timeout(Duration::from_secs(60), async {
            loop {
                if self.automation.is_ragexe_focused() {
                    return Ok::<(), LoginError>(());
                }
                sleep(Duration::from_millis(500)).await;
            }
        })
        .await;

        match result {
            Ok(Ok(())) => {
                Ok(())
            }
            _ => {
                Err(LoginError::Timeout("ragexe.exe focus".to_string()))
            }
        }
    }

    /// Find image with timeout, focus check, and optional mouse movement
    async fn find_image_with_timeout<F>(
        &mut self,
        image_name: &str,
        threshold: f64,
        timeout_secs: u64,
        _report_progress: &mut F,
        move_mouse: bool,
    ) -> Result<ImageMatch, LoginError>
    where
        F: FnMut(String),
    {
        let image_path = format!("{}\\{}", self.resources_path, image_name);

        let result = timeout(Duration::from_secs(timeout_secs), async {
            loop {
                // Check if focus is lost
                if !self.automation.is_ragexe_focused() {
                    return Err(LoginError::RagexeFocusLost);
                }

                // Move mouse away if requested (for PIN search)
                if move_mouse {
                    if let Err(e) = self.automation.move_mouse_away() {
                        return Err(LoginError::AutomationError(format!("Failed to move mouse: {}", e)));
                    }
                }

                // Try to find image
                match self.automation.find_image_on_screen(&image_path, threshold) {
                    Ok(Some(image_match)) => {
                        return Ok(image_match);
                    }
                    Ok(None) => {
                        // Not found yet, continue waiting
                    }
                    Err(e) => {
                        return Err(LoginError::AutomationError(e.to_string()));
                    }
                }

                sleep(Duration::from_millis(500)).await;
            }
        })
        .await;

        match result {
            Ok(Ok(image_match)) => {
                Ok(image_match)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(LoginError::Timeout(image_name.to_string())),
        }
    }

    /// Step 2: Enter login credentials
    async fn enter_login_credentials<F>(
        &mut self,
        credentials: &LoginCredentials,
        report_progress: &mut F,
    ) -> Result<(), LoginError>
    where
        F: FnMut(String),
    {
        report_progress("Searching for login screen...".to_string());

        // Find login page
        let login_match = self
            .find_image_with_timeout("login_page.png", 0.9, 60, report_progress, false)
            .await?;

        // Click below the login image (bottom center + 5px)
        let click_x = login_match.x + (login_match.width / 2);
        let click_y = login_match.y + login_match.height + 5;

        self.automation
            .click_at(click_x, click_y)
            .map_err(|e| LoginError::AutomationError(e.to_string()))?;

        sleep(Duration::from_millis(500)).await;

        // Type email using paste (much faster)
        report_progress("Entering email...".to_string());
        self.automation
            .paste_text(&credentials.email)
            .map_err(|e| LoginError::AutomationError(e.to_string()))?;

        sleep(Duration::from_millis(200)).await;

        // Press Tab to move to password field
        self.automation
            .press_tab()
            .map_err(|e| LoginError::AutomationError(e.to_string()))?;

        sleep(Duration::from_millis(200)).await;

        // Type password using paste (much faster)
        self.automation
            .paste_text(&credentials.password)
            .map_err(|e| LoginError::AutomationError(e.to_string()))?;

        sleep(Duration::from_millis(200)).await;

        // Press Enter to submit
        self.automation
            .press_enter()
            .map_err(|e| LoginError::AutomationError(e.to_string()))?;

        sleep(Duration::from_millis(1000)).await;

        Ok(())
    }

    /// Step 3: Enter OTP
    async fn enter_otp<F>(
        &mut self,
        otp: &str,
        report_progress: &mut F,
    ) -> Result<(), LoginError>
    where
        F: FnMut(String),
    {
        report_progress("Waiting for OTP screen...".to_string());

        // Find OTP page
        self.find_image_with_timeout("otp_page.png", 0.9, 60, report_progress, false)
            .await?;

        sleep(Duration::from_millis(300)).await;

        // Type OTP using paste (much faster)
        report_progress("Entering OTP...".to_string());
        self.automation
            .paste_text(otp)
            .map_err(|e| LoginError::AutomationError(e.to_string()))?;

        sleep(Duration::from_millis(200)).await;

        // Press Enter
        self.automation
            .press_enter()
            .map_err(|e| LoginError::AutomationError(e.to_string()))?;

        Ok(())
    }

    /// Step 4: Select server
    async fn select_server<F>(
        &mut self,
        report_progress: &mut F,
    ) -> Result<(), LoginError>
    where
        F: FnMut(String),
    {
        report_progress("Waiting for server selection...".to_string());

        // Find server select page
        self.find_image_with_timeout("server_select.png", 0.9, 60, report_progress, false)
            .await?;

        sleep(Duration::from_millis(300)).await;

        // Press Enter to select default server
        self.automation
            .press_enter()
            .map_err(|e| LoginError::AutomationError(e.to_string()))?;

        Ok(())
    }

    /// Step 5: Enter PIN
    async fn enter_pin<F>(
        &mut self,
        pin: &str,
        report_progress: &mut F,
    ) -> Result<(), LoginError>
    where
        F: FnMut(String),
    {
        report_progress("Waiting for PIN screen...".to_string());

        // Find PIN select page
        self.find_image_with_timeout("pin_select.png", 0.9, 60, report_progress, false)
            .await?;

        sleep(Duration::from_millis(200)).await;

        report_progress("Entering PIN...".to_string());

        // Click each digit of the PIN
        for (i, digit) in pin.chars().enumerate() {
            if !digit.is_ascii_digit() {
                return Err(LoginError::AutomationError(
                    format!("PIN contains non-numeric characters: {}", pin),
                ));
            }

            let digit_image = format!("{}.bmp", digit);

            // Try to find digit WITHOUT moving mouse first (faster)
            let digit_path = format!("{}\\{}", self.resources_path, digit_image);
            let quick_search = self.automation.find_image_on_screen(&digit_path, 0.9)
                .map_err(|e| LoginError::AutomationError(e.to_string()))?;
            
            let digit_match = if let Some(m) = quick_search {
                // Found without moving mouse!
                m
            } else {
                // Not found, move mouse away and search with timeout
                self.find_image_with_timeout(&digit_image, 0.9, 10, report_progress, true)
                    .await?
            };

            // Click center of digit
            let click_x = digit_match.x + (digit_match.width / 2);
            let click_y = digit_match.y + (digit_match.height / 2);

            self.automation
                .click_at(click_x, click_y)
                .map_err(|e| LoginError::AutomationError(e.to_string()))?;

            // On last digit, also find OK button in the SAME screen capture
            if i == pin.len() - 1 {
                sleep(Duration::from_millis(200)).await;
                
                report_progress("Confirming PIN...".to_string());
                
                // Search for OK button without moving mouse (reuse current screen state)
                let ok_path = format!("{}\\ok_pin.png", self.resources_path);
                let ok_result = self.automation.find_image_on_screen(&ok_path, 0.85)
                    .map_err(|e| LoginError::AutomationError(e.to_string()))?;
                
                if let Some(ok_match) = ok_result {
                    // Found OK button, click it
                    let ok_x = ok_match.x + (ok_match.width / 2);
                    let ok_y = ok_match.y + (ok_match.height / 2);
                    
                    self.automation
                        .click_at(ok_x, ok_y)
                        .map_err(|e| LoginError::AutomationError(e.to_string()))?;
                } else {
                    // Fallback: move mouse away ONCE and search ONE more time
                    self.automation.move_mouse_away()
                        .map_err(|e| LoginError::AutomationError(e.to_string()))?;
                    
                    sleep(Duration::from_millis(100)).await;
                    
                    let ok_result_retry = self.automation.find_image_on_screen(&ok_path, 0.85)
                        .map_err(|e| LoginError::AutomationError(e.to_string()))?;
                    
                    if let Some(ok_match) = ok_result_retry {
                        let ok_x = ok_match.x + (ok_match.width / 2);
                        let ok_y = ok_match.y + (ok_match.height / 2);
                        
                        self.automation
                            .click_at(ok_x, ok_y)
                            .map_err(|e| LoginError::AutomationError(e.to_string()))?;
                    } else {
                        // Still not found after moving mouse, STOP
                        return Err(LoginError::AutomationError(
                            "OK button not found after moving mouse".to_string()
                        ));
                    }
                }
            } else {
                sleep(Duration::from_millis(300)).await;
            }
        }

        sleep(Duration::from_millis(500)).await;

        Ok(())
    }
}
