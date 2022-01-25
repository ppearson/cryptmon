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

// Note: I found all sorts of different and apparently contradictary (in terms of use statements)
//       examples of how to use lettre, even for just 0.9.x versions, so I don't think
//       the API is that stable, as some examples for 0.9.4 didn't seem to exist with 0.9.6, so...
use lettre::smtp::authentication::Credentials;
use lettre::{SmtpClient, Transport};

pub struct AlertProviderSMTPMail {
    pub to_address:     String,

    pub smtp_server:    String,
    pub smtp_username:  String,
    pub smtp_password:  String,
}

impl AlertProviderSMTPMail {

    pub fn new_configure(config: &AlertProviderConfig) -> Option<AlertProviderSMTPMail> {
        let to_address = config.get_param_as_string("toAddress");
        if to_address.is_none() {
            eprintln!("Error: 'mailSMTP' Alert provider was not configured with a 'toAddress' param.");
            return None;
        }

        let smtp_server = config.get_param_as_string("smtpServer");
        if smtp_server.is_none() {
            eprintln!("Error: 'mailSMTP' Alert provider was not configured with an 'smtpServer' param.");
            return None;
        }
        let smtp_username = config.get_param_as_string("smtpUsername");
        if smtp_username.is_none() {
            eprintln!("Error: 'mailSMTP' Alert provider was not configured with an 'smtpUsername' param.");
            return None;
        }
        let smtp_password = config.get_param_as_string("smtpPassword");
        if smtp_password.is_none() {
            eprintln!("Error: 'mailSMTP' Alert provider was not configured with an 'smtpPassword' param.");
            return None;
        }

        return Some(AlertProviderSMTPMail{ to_address: to_address.unwrap(), smtp_server: smtp_server.unwrap(),
                                            smtp_username: smtp_username.unwrap(),
                                            smtp_password: smtp_password.unwrap() });
    }
}

impl AlertProvider for AlertProviderSMTPMail {
    fn send_alert(&self, message_params: AlertMessageParams) -> Result<(), SendAlertError> {

//        let smtp_port = 587u16;

        let email: lettre_email::Email = lettre_email::Email::builder()
            .to(self.to_address.to_string())
            .from(self.smtp_username.to_string())
            .subject(message_params.subject)
            .text(message_params.message)
            .build().unwrap();
        
        let mut client = SmtpClient::new_simple(&self.smtp_server).unwrap()
                .credentials(Credentials::new(self.smtp_username.to_string(), self.smtp_password.to_string()))
                .transport();

        // TODO: error checking...
        client.send(email.into()).unwrap();
            
        return Ok(());
    }

}
