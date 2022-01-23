/*
 Cryptmon
 Copyright 2022 Peter Pearson.
 Licensed under the Apache License, Version 2.0 (the "License");
 You may not use this file except in compliance with the License.
 You may obtain a copy of the License at
 http://www.apache.org/licenses/LICENSE-2.0
 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
 ---------
*/

use std::fmt;

#[derive(Clone, Debug)]
pub struct AlertMessageParams {
    pub subject:            String,
    pub message:            String,
}

impl AlertMessageParams {
    pub fn new(subject: &str, message: &str) -> AlertMessageParams {
        return AlertMessageParams{ subject: subject.to_string(), message: message.to_string() };
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum SendAlertError {
    ConfigError(String),
    CantConnect(String),
    NoResponse(String),
    AuthenticationError(String),
    InvalidAPIParams(String),
    ParseError(String),
    OtherError(String),
    NotImplemented
}

impl fmt::Display for SendAlertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SendAlertError::ConfigError(ref err) => write!(f, "Configuration Error: {}", err),
            SendAlertError::CantConnect(ref err) => write!(f, "Can't Connect: {}", err),
            SendAlertError::NoResponse(ref err) => write!(f, "No Response from API server: {}", err),
            SendAlertError::AuthenticationError(ref err) => write!(f, "Authentication Error: {}", err),
            SendAlertError::InvalidAPIParams(ref err) => write!(f, "Invalid API params provided: {}", err),
            SendAlertError::ParseError(ref err) => write!(f, "Error parsing response: {}", err),
            SendAlertError::OtherError(ref err) => write!(f, "Error with API call: {}", err),
            SendAlertError::NotImplemented => write!(f, "Not implemented"),
        }
    }
}

pub trait AlertProvider {
    fn send_alert(&self, _message_params: AlertMessageParams) -> Result<(), SendAlertError> {
        return Err(SendAlertError::NotImplemented);
    }
}
