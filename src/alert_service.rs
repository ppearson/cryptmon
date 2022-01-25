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

use crate::config::{Config, AlertConfig};

use crate::price_provider::{PriceProvider, PriceProviderParams, CoinPriceItem};
use crate::formatting_helpers::{smart_format};

use crate::alert_provider::{AlertProvider, AlertMessageParams};//, SendAlertError};

#[cfg(feature = "smtp")]
use crate::alert_provider_smtp_mail::{AlertProviderSMTPMail};

use crate::alert_provider_pushsafer::{AlertProviderPushSafer};
use crate::alert_provider_simplepush::{AlertProviderSimplePush};
use crate::alert_provider_textbelt::{AlertProviderTextbelt};

use std::rc::Rc;

use std::collections::BTreeMap;

use chrono::{Local, Duration};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AlertTriggerType {
    PriceLessThan,
    PriceLessThanEqualTo,
    PriceGreaterThanEqualTo,
    PriceGreaterThan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum AlertAction {
    PrintMessage,
    ShowNotification,
    RunCommand(String),
    RunProvider(String),
}

#[derive(Clone)]
struct AlertItem {
    // note: these values are always lower-case here...
    pub coin_symbol:            String,
    pub trigger_type:           AlertTriggerType,
    pub trigger_price:          f64,

    pub action:                 AlertAction,

    // this is only set for alert action types that use providers
    pub alert_provider: Option<Rc<dyn AlertProvider>>,
}

//#[derive(Clone, Debug)]
struct InternalAlertState {
    pub main_alert: AlertItem,

    // additional state...
    pub last_price:     f64,

    pub has_triggered:  bool,

    pub previous_alert_watermark: Option<f64>,
    
    pub watermark_trip_sleep_until: Option<chrono::DateTime<Local>>,

    pub sleep_until:    chrono::DateTime<Local>,
}

pub struct AlertService {
    // our copy of the config...
    config:             Config,

    // what was originally provided to the PriceProvider to configure it.... possibly with deferred modifications (i.e.)
    // wanted_coin_symbols...
    price_provider_params:  PriceProviderParams,
    price_provider:     Box<dyn PriceProvider>,

    // TODO: not sure about this, but something like this needs to happen somewhere, so let's at least get the basics working...
    alert_providers:    BTreeMap<String, Rc<dyn AlertProvider>>,

    alert_items:        Vec<InternalAlertState>,
}


impl AlertService {
    pub fn new(config: &Config, price_provider_params: &PriceProviderParams, price_provider: Box<dyn PriceProvider>) -> Option<AlertService> {
        let mut alert_service = AlertService{ config: config.clone(), price_provider_params: price_provider_params.clone(),
                                          price_provider, alert_providers: BTreeMap::new(),
                                          alert_items: Vec::with_capacity(0) };
        
        // register and configure any enabled alert providers
        // TODO: not sure about the best way of doing this...
        alert_service.register_alert_provider("pushsafer", &config.alert_config);
        alert_service.register_alert_provider("simplepush", &config.alert_config);
        alert_service.register_alert_provider("textbelt", &config.alert_config);

        #[cfg(feature = "smtp")]
        if !alert_service.register_alert_provider("mailSMTP", &config.alert_config) {
            eprintln!("Error: Support for the 'mailSMTP' provider was not compiled into this binary.");
            return None;
        }

        let alert_items = alert_service.process_alert_config_items(&config.alert_config);
        if alert_items.is_none() {
            eprintln!("Error: no alerts were found to be registered for coins to monitor in cryptmon.ini...");
            return None;
        }

        let mut wanted_coins = Vec::with_capacity(0);

        for alert_item in alert_items.unwrap() {
            wanted_coins.push(alert_item.coin_symbol.to_ascii_lowercase());
            let internal_alert_state = InternalAlertState{ main_alert: alert_item, last_price: 0.0,
                                                           has_triggered: false,
                                                           previous_alert_watermark: None,
                                                           watermark_trip_sleep_until: None,
                                                           sleep_until: Local::now() };

            alert_service.alert_items.push(internal_alert_state);
        }

        // lazily update the price provider with the symbols we want by reconfiguring it again...
        // Not amazingly happy about this, but I'm less happy with alternatives in this chicken-and-egg situation...
        alert_service.price_provider_params.wanted_coin_symbols = wanted_coins;
        let mut_provider = &mut alert_service.price_provider;
        mut_provider.configure(&alert_service.price_provider_params);

        return Some(alert_service);
    }

    fn register_alert_provider(&mut self, name: &str, config: &AlertConfig) -> bool {
        // if we don't have a config for the name, we assume it's not enabled, so ignore it and
        // don't register it...
        if let Some(conf) = config.alert_provider_configs.get(name) {
            // TODO: something less hard-coded than this...
            if name == "pushsafer" {
                if let Some(provider) = AlertProviderPushSafer::new_configure(conf) {
                    self.alert_providers.insert(name.to_string(), Rc::new(provider));
                    return true;
                }
            }
            else if name == "simplepush" {
                if let Some(provider) = AlertProviderSimplePush::new_configure(conf) {
                    self.alert_providers.insert(name.to_string(), Rc::new(provider));
                    return true;
                }
            }
            else if name == "textbelt" {
                if let Some(provider) = AlertProviderTextbelt::new_configure(conf) {
                    self.alert_providers.insert(name.to_string(), Rc::new(provider));
                    return true;
                }
            }
            else if name == "mailSMTP" {
                #[cfg(not(feature = "smtp"))]
                return false;

                #[cfg(feature = "smtp")]
                if let Some(provider) = AlertProviderSMTPMail::new_configure(conf) {
                    self.alert_providers.insert(name.to_string(), Rc::new(provider));
                    return true;
                }
            }
        }

        eprintln!("Error: can't register or configure Alert provider: '{}'.", name);

        return false;
    }

    fn process_alert_config_items(&mut self, alert_config: &AlertConfig) -> Option<Vec<AlertItem>> {
        let mut alert_items = Vec::new();

        for alert_conf in &alert_config.alert_config_strings {
            let start_parenth = alert_conf.find('(');
            let end_parenth = alert_conf.rfind(')');
            if start_parenth.is_none() || end_parenth.is_none() {
                continue;
            }

            let param_contents = &alert_conf[start_parenth.unwrap()+1..end_parenth.unwrap()];
            let params: Vec<&str> = param_contents.split(',').map(|x| x.trim()).collect();
            if params.len() != 4 {
                continue;
            }

            let symbol = params[0];
            let alert_trigger_type = match params[1] {
                "<" =>  Some(AlertTriggerType::PriceLessThan),
                "<=" => Some(AlertTriggerType::PriceLessThanEqualTo),
                ">" =>  Some(AlertTriggerType::PriceGreaterThan),
                ">=" => Some(AlertTriggerType::PriceGreaterThanEqualTo),
                _   =>  None,
            };

            if symbol.is_empty() || alert_trigger_type.is_none() {
                continue;
            }

            let alert_trigger_type = alert_trigger_type.unwrap();

            let price_value = params[2].parse::<f64>();
            if price_value.is_err() {
                continue;
            }
            let price_value = price_value.unwrap();

            let alert_action;
            let alert_action_string = params[3];
            if alert_action_string == "print" {
                alert_action = Some(AlertAction::PrintMessage);
            }
            else if alert_action_string == "showNotification" {
                alert_action = Some(AlertAction::ShowNotification);
                
                #[cfg(not(feature = "notifications"))]
                eprintln!("Error: Notifications support is not compiled into this binary. Please enable the feature.");
            }
            else if alert_action_string.starts_with("runCommand") && alert_action_string.contains(':') {
                // TODO: - split the string
                alert_action = Some(AlertAction::RunCommand("".to_string()));
            }
            else {
                // it's likely a generic alert provider...
                alert_action = Some(AlertAction::RunProvider(alert_action_string.to_string()));
            }

            if alert_action.is_none() {
                continue;
            }
            let alert_action = alert_action.unwrap();

            let mut alert_provider: Option<Rc<dyn AlertProvider>> = None;
            if let AlertAction::RunProvider(provider_name) = &alert_action {
                // it's a generic provider, so see if we have that registered (and configured)...
                if let Some(provider) = self.alert_providers.get(provider_name) {
                    alert_provider = Some(Rc::clone(provider));
                }
                else {
                    eprintln!("Error: can't find registered and configured Alert Provider called '{}'", provider_name);
                    continue;
                }
            }

            let new_alert = AlertItem{ coin_symbol: symbol.to_ascii_lowercase(), trigger_type: alert_trigger_type,
                                       trigger_price: price_value, action: alert_action, alert_provider };
            alert_items.push(new_alert);
        }

        if alert_items.is_empty() {
            return None;
        }

        return Some(alert_items);
    }

    pub fn run(&mut self) {

        if self.alert_items.is_empty() {
            eprintln!("Error: No alert items found.");
            return;
        }

        loop {
            let results = self.price_provider.get_current_prices();

            if let Err(err) = results {
                eprintln!("Error getting price results: {}", err.to_string());

                // TODO: maybe we don't want to wait as long first time, but want a backoff of some sort?
                std::thread::sleep(std::time::Duration::from_secs(self.config.alert_config.check_period));

                continue;
            }

            let prices = results.unwrap();

            let local_time = Local::now();

            // brute-force it for now...
            for mut alert in &mut self.alert_items {

                // check we should validate it
                if alert.has_triggered && alert.sleep_until > local_time {
                    // skip it this time around, as we don't want alerts until the general sleep_until time has expired
                    continue;
                }

                let current_price = get_price_for_symbol(&alert.main_alert.coin_symbol, &prices);
                if current_price.is_none() {
                    eprintln!("Error: Price for symbol: {} was not found", alert.main_alert.coin_symbol);
                    continue;
                }

                let current_price = current_price.unwrap();

                // TODO: see if we can simplify/condense this - use match which defines a closure?...
                let m_alert = &alert.main_alert;
                let alert_triggered = should_alert_trigger(m_alert.trigger_type, m_alert.trigger_price, current_price);
                
                if alert_triggered {
                    let mut should_show_alert = true;

//                        let cached_last_price = alert.last_price;

                    // update the last price here, so it's done for all code paths...
                    alert.last_price = current_price;

                    // see if we should sleep due to general sleep...
                    if local_time < alert.sleep_until {
                        // we're still sleeping, so don't...
                        should_show_alert = false;
                    }

                    // otherwise, see if we should check watermark trip sleep...
                    if should_show_alert && self.config.alert_config.watermark_trip_sleep_enabled {
                        if let Some(watermark_sleep_until) = alert.watermark_trip_sleep_until {
                            if local_time < watermark_sleep_until {
                                let prev_watermark_val = alert.previous_alert_watermark.unwrap();

                                // in theory we're still sleeping for the watermark trip sleep for this
                                // alert, but if the watermark for this alert has been tripped,
                                // we can alert.
                                // So basically, only set should_show_alert = false if we haven't tripped the
                                // existing watermark

                                let should_watermark_trigger = should_alert_trigger_watermark(m_alert.trigger_type, prev_watermark_val, current_price);
                                if !should_watermark_trigger {
                                    should_show_alert = false;
                                }
                            }
                            else {
                                // otherwise, the watermark trip sleep has elapsed, and we want to "Action" the Alert...
                            }
                        }
                    }

                    // TODO: think about what to do if multiple alerts trigger: combine the messages? Concat them based
                    //       of the providers? i.e. for all alerts which triggered sharing the same provider, combine them?
                    //       It's very likely we want to do something like this in the future for alerts where you might
                    //       pay (i.e. SMS notifications), and wouldn't want duplicate messages for multiple coins at the
                    //       same instant. Or similarly, where free notification plans for push notifications / SMSs
                    //       provide a limited number of free API calls per month.

                    if should_show_alert {
                        alert.sleep_until = local_time.checked_add_signed(Duration::seconds(self.config.alert_config.general_sleep_period as i64)).unwrap();

                        if self.config.alert_config.watermark_trip_sleep_enabled {
                            alert.previous_alert_watermark = Some(current_price);
                            let watermark_sleep_until = local_time.checked_add_signed(Duration::seconds(self.config.alert_config.watermark_trip_sleep_period as i64)).unwrap();
                            alert.watermark_trip_sleep_until = Some(watermark_sleep_until);
                        }

                        let alert_message = format!("Coin: {} is at price: {}.", m_alert.coin_symbol.to_ascii_uppercase(),
                                                        &smart_format(current_price));
                        if m_alert.action == AlertAction::PrintMessage {
                            eprintln!("{}", alert_message);
                        }
                        else if m_alert.action == AlertAction::ShowNotification {
                            #[cfg(feature = "notifications")]
                            notifica::notify("Cryptmon Price Alert", &alert_message).unwrap();
                        }
                        else if let AlertAction::RunProvider(prov_name) = &m_alert.action {
                            if let Some(provider) = &m_alert.alert_provider {
                                let alert_message = AlertMessageParams::new("Cryptmon Price Alert", &alert_message);
                                // TODO: maybe we want to try and do this asynchronously at some point, although it might
                                //       just be easier to set a pretty short connection timeout as a config option,
                                //       and providers can use that?
                                let res = provider.send_alert(alert_message);
                                if res.is_err() {
                                    eprintln!("Error: Error sending alert with provider: '{}'", prov_name);
                                    // TODO: If we weren't successful in sending the alert/notification with the provider,
                                    // we probably want to assume it wasn't sent, and thus maybe not activate any sleep_until
                                    // in this situation until we know we've managed to send an alert?
                                }
                            }
                        }
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_secs(self.config.alert_config.check_period));
        }
    }
}

fn should_alert_trigger(trigger_type: AlertTriggerType, trigger_value: f64, actual_value: f64) -> bool {
    let mut alert_triggered = false;

    if trigger_type == AlertTriggerType::PriceGreaterThan && actual_value > trigger_value {
        alert_triggered = true;
    }
    else if trigger_type == AlertTriggerType::PriceGreaterThanEqualTo && actual_value >= trigger_value {
        alert_triggered = true;
    }
    else if trigger_type == AlertTriggerType::PriceLessThan && actual_value < trigger_value {
        alert_triggered = true;
    }
    else if trigger_type == AlertTriggerType::PriceLessThanEqualTo && actual_value <= trigger_value {
        alert_triggered = true;
    }

    return alert_triggered;
}

// this version is used for watermarks, and so maps >= to >, and <= to <, as it doesn't make sense
// to trip the alerts on watermarks being equal to...
fn should_alert_trigger_watermark(trigger_type: AlertTriggerType, watermark_value: f64, actual_value: f64) -> bool {
    let mut alert_triggered = false;

    if (trigger_type == AlertTriggerType::PriceGreaterThan || trigger_type == AlertTriggerType::PriceGreaterThanEqualTo)
                                 && actual_value > watermark_value {
        alert_triggered = true;
    }
    else if (trigger_type == AlertTriggerType::PriceLessThan || trigger_type == AlertTriggerType::PriceLessThanEqualTo)
                                 && actual_value < watermark_value {
        alert_triggered = true;
    }
    
    return alert_triggered;
}

fn get_price_for_symbol(symbol: &str, prices: &[CoinPriceItem]) -> Option<f64> {
    for price in prices {
        if price.symbol.to_ascii_lowercase() == symbol {
            return Some(price.current_price);
        }
    }

    return None;
}
