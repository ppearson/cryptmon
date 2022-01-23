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
struct TextbeltTextResponse {
    success:                bool,
    error:                  Option<String>,
    text_id:                Option<String>,
    quota_remaining:        u32,
}

#[derive(Clone, Debug)]
pub struct AlertProviderTextbelt {
    pub api_key:        String,
    pub phone_number:   String,
}

impl AlertProviderTextbelt {
    pub fn new_configure(config: &AlertProviderConfig) -> Option<AlertProviderTextbelt> {
        let mut api_key = "textbelt".to_string();
        if let Some(custom_key) = config.get_param_as_string("API_KEY") {
            api_key = custom_key;
        }

        let phone_number = config.get_param_as_string("phoneNumber");
        if phone_number.is_none() {
            eprintln!("Error: 'textbelt' Alert provider was not configured with a 'phoneNumber' param.");
            return None;
        }
        return Some(AlertProviderTextbelt{ api_key, phone_number: phone_number.unwrap() });
    }
}

impl AlertProvider for AlertProviderTextbelt {
    fn send_alert(&self, message_params: AlertMessageParams) -> Result<(), SendAlertError> {

        let json_value = ureq::json!({
            "phone": &self.phone_number,
            "message": &message_params.message,
            "key": &self.api_key,
        });

        let resp = ureq::post("https://textbelt.com/text")
            .send_json(json_value);

        // TODO: there's an insane amount of boilerplate error handling and response
        //       decoding going on here... Try and condense it...
        
        if resp.is_err() {
            match resp.err() {
                Some(Error::Status(code, response)) => {
                    // server returned an error code we weren't expecting...
                    match code {
                        401 => {
                            eprintln!("Error: authentication error with https://textbelt.com/text: {}", response.into_string().unwrap());
                            return Err(SendAlertError::AuthenticationError("".to_string()));
                        },
                        404 => {
                            eprintln!("Error: Not found response from https://textbelt.com/text: {}", response.into_string().unwrap());
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
        
        let resp_string = resp.unwrap().into_string().unwrap();
        let response_struct = serde_json::from_str::<TextbeltTextResponse>(&resp_string);
        if let Err(_err) = response_struct {
            return Err(SendAlertError::ParseError(format!("Can't parse: {}", resp_string)));
        }
        let response_struct = response_struct.unwrap();
        if !response_struct.success {
            return Err(SendAlertError::OtherError(response_struct.error.unwrap()));
        }
            
        return Ok(());
    }

}
