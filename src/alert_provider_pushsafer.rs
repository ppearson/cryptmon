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

use crate::config::{AlertProviderConfig};
use crate::alert_provider::{AlertProvider, SendAlertError, AlertMessageParams};

use ureq::Error;

#[derive(Clone, Debug)]
pub struct AlertProviderPushSafer {
    pub private_key:    String,
    pub device_id:      String,
}

impl AlertProviderPushSafer {
    pub fn new_configure(config: &AlertProviderConfig) -> Option<AlertProviderPushSafer> {
        let private_key = config.get_param_as_string("privateKey");
        if private_key.is_none() {
            eprintln!("Error: 'pushsafe' Alert provider was not configured with a 'privateKey' param.");
            return None;
        }

        let device_id = config.get_param_as_string("deviceID");
        if device_id.is_none() {
            eprintln!("Error: 'pushsafe' Alert provider was not configured with a 'deviceID' param.");
            return None;
        }
        return Some(AlertProviderPushSafer{ private_key: private_key.unwrap(), device_id: device_id.unwrap() });
    }
}

impl AlertProvider for AlertProviderPushSafer {
    fn send_alert(&self, message_params: AlertMessageParams) -> Result<(), SendAlertError> {

        let resp = ureq::post("https://www.pushsafer.com/api")
            .query("k", &self.private_key)
            .query("t", &message_params.subject)
            .query("m", &message_params.message)
            .call();

        // TODO: there's an insane amount of boilerplate error handling and response
        //       decoding going on here... Try and condense it...
        
        if resp.is_err() {
            match resp.err() {
                Some(Error::Status(code, response)) => {
                    // server returned an error code we weren't expecting...
                    match code {
                        401 => {
                            eprintln!("Error: authentication error with https://www.pushsafer.com/api: {}", response.into_string().unwrap());
                            return Err(SendAlertError::AuthenticationError("".to_string()));
                        },
                        404 => {
                            eprintln!("Error: Not found response from https://www.pushsafer.com/api: {}", response.into_string().unwrap());
                            return Err(SendAlertError::OtherError("".to_string()));
                        }
                        _ => {
                            
                        }
                    }
                    eprintln!("Error sending text message: code: {}, resp: {:?}", code, response);
                },
                Some(e) => {
                    eprintln!("Error sending text message: {:?}", e);
                }
                _ => {
                    // some sort of transport/io error...
                    eprintln!("Error sending text message: ");
                }
            }
            return Err(SendAlertError::OtherError("".to_string()));
        }

        let resp = resp.unwrap();
        // PushSafer also returns some errors as 2XX codes, so we need to handle those as errors as well...
        match &resp.status() {
            250         => {
                eprintln!("Error: Invalid privateKey provided {}", resp.into_string().unwrap());
                return Err(SendAlertError::OtherError("Invalid private key".to_string()));
            },
            255         => {
                eprintln!("Error: Invalid privateKey provided or empty message {}", resp.into_string().unwrap());
                return Err(SendAlertError::OtherError("Invalid private key or empty message".to_string()));
            },
            260         => {
                eprintln!("Error: empty message {}", resp.into_string().unwrap());
                return Err(SendAlertError::OtherError("empty message".to_string()));
            },
            270         => {
                eprintln!("Error: Invalid device ID... {}", resp.into_string().unwrap());
                return Err(SendAlertError::OtherError("Invalid device ID".to_string()));
            }
            280         => {
                eprintln!("Error: Insufficient API calls remaining... {}", resp.into_string().unwrap());
                return Err(SendAlertError::OtherError("Insufficient API calls remaining".to_string()));
            },
            200         => {

            },
            _           => {
                eprintln!("Error: Other error... {}", resp.into_string().unwrap());
                return Err(SendAlertError::OtherError("Other error".to_string()));
            }
        }
        
        let _resp_string = resp.into_string().unwrap();
            
        return Ok(());
    }

}
