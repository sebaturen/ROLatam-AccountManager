use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "auth_temp")]
    AuthTemp { temp_key: String },
    
    #[serde(rename = "auth_permanent")]
    AuthPermanent { 
        permanent_key: String,
        client_id: String 
    },
    
    #[serde(rename = "reporting")]
    Reporting { message: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "auth_success")]
    AuthSuccess {
        permanent_key: String,
        client_id: String,
    },
    
    #[serde(rename = "auth_error")]
    AuthError { message: String },
    
    #[serde(rename = "login")]
    Login {
        email: String,
        password: String,
        pin: String,
        otp: String,
    },
}

impl ClientMessage {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl ServerMessage {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
