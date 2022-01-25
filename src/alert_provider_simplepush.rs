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

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[derive(Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct SimplePushResponse {
    status:     String,
}

#[derive(Clone, Debug)]
pub struct AlertProviderSimplePush {
    pub key:    String,
}

impl AlertProviderSimplePush {
    pub fn new_configure(config: &AlertProviderConfig) -> Option<AlertProviderSimplePush> {
        let key = config.get_param_as_string("key");
        if key.is_none() {
            eprintln!("Error: 'simplepush' Alert provider was not configured with a 'key' param.");
            return None;
        }

        return Some(AlertProviderSimplePush{ key: key.unwrap() });
    }
}

impl AlertProvider for AlertProviderSimplePush {
    fn send_alert(&self, message_params: AlertMessageParams) -> Result<(), SendAlertError> {
        let json_value = ureq::json!({
            "key": &self.key,
            "title": &message_params.subject,
            "msg": &message_params.message,
            "event": "event",
        });

        let resp = ureq::post("https://api.simplepush.io/send")
            .send_json(json_value);

        // TODO: there's an insane amount of boilerplate error handling and response
        //       decoding going on here... Try and condense it...
        
        if resp.is_err() {
            match resp.err() {
                Some(Error::Status(code, response)) => {
                    // server returned an error code we weren't expecting...
                    match code {
                        401 => {
                            eprintln!("Error: authentication error with https://api.simplepush.io/send: {}", response.into_string().unwrap());
                            return Err(SendAlertError::AuthenticationError("".to_string()));
                        },
                        404 => {
                            eprintln!("Error: Not found response from https://api.simplepush.io/send: {}", response.into_string().unwrap());
                            return Err(SendAlertError::OtherError("".to_string()));
                        }
                        _ => {
                            
                        }
                    }
                    eprintln!("Error sending notification request: code: {}, resp: {:?}", code, response);
                },
                Some(e) => {
                    eprintln!("Error sending notification request: {:?}", e);
                }
                _ => {
                    // some sort of transport/io error...
                    eprintln!("Error sending notification request: ");
                }
            }
            return Err(SendAlertError::OtherError("".to_string()));
        }
        
        let resp_string = resp.unwrap().into_string().unwrap();
        let response_struct = serde_json::from_str::<SimplePushResponse>(&resp_string);
        if let Err(_err) = response_struct {
            return Err(SendAlertError::ParseError(format!("Can't parse: {}", resp_string)));
        }
        let response_struct = response_struct.unwrap();
        
        if response_struct.status != "OK" {
            return Err(SendAlertError::OtherError("Error sending notification request to simplepush.io".to_string()));
        }
            
        return Ok(());
    }

}
