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

use chrono::{Local};

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
    RunCommand(String)
}

#[derive(Clone, Debug, PartialEq)]
struct AlertItem {
    // note: these values are always lower-case here...
    pub coin_symbol:            String,
    pub trigger_type:           AlertTriggerType,
    pub trigger_price:          f64,

    pub action:                 AlertAction,
}

#[derive(Clone, Debug)]
struct InternalAlertState {
    pub main_alert: AlertItem,

    // additional state...
    pub last_price:     f64,

    pub has_triggered:  bool,

//    pub previous_alert_watermark: Option<f64>,

    pub sleep_until:    chrono::DateTime<Local>,
}

pub struct AlertService {
    // our copy of the config...
    config:             Config,

    // what was originally provided to the PriceProvider to configure it.... possibly with deferred modifications (i.e.)
    // wanted_coin_symbols...
    price_provider_params:  PriceProviderParams,
    price_provider:     Box<dyn PriceProvider>,

    alert_items:        Vec<InternalAlertState>,
}


impl AlertService {
    pub fn new(config: &Config, price_provider_params: &PriceProviderParams, price_provider: Box<dyn PriceProvider>) -> Option<AlertService> {
        let mut alert_service = AlertService{ config: config.clone(), price_provider_params: price_provider_params.clone(),
                                          price_provider,
                                          alert_items: Vec::with_capacity(0) };

        let alert_items = AlertService::process_alert_config_items(&config.alert_config);
        if alert_items.is_none() {
            return None;
        }

        let mut wanted_coins = Vec::with_capacity(0);

        for alert_item in alert_items.unwrap() {
            wanted_coins.push(alert_item.coin_symbol.to_ascii_lowercase());
            let internal_alert_state = InternalAlertState{ main_alert: alert_item, last_price: 0.0,
                                                           has_triggered: false,
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

    fn process_alert_config_items(alert_config: &AlertConfig) -> Option<Vec<AlertItem>> {
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

            let alert_action = match params[3] {
                "print"        => Some(AlertAction::PrintMessage),
                "runCommand"   => Some(AlertAction::RunCommand("".to_string())),
                _              => None,
            };

            if alert_action.is_none() {
                continue;
            }
            let alert_action = alert_action.unwrap();

            let new_alert = AlertItem{ coin_symbol: symbol.to_ascii_lowercase(), trigger_type: alert_trigger_type,
                                       trigger_price: price_value, action: alert_action };
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
            }
            else {
                let prices = results.unwrap();

                let local_time = Local::now();

                // brute-force it for now...
                for alert in &self.alert_items {

                    // check we should validate it
                    if alert.has_triggered && alert.sleep_until > local_time {
                        // skip it this time around, as we don't want alerts until the sleep_until time has expired
                        continue;
                    }

                    let current_price = get_price_for_symbol(&alert.main_alert.coin_symbol, &prices);
                    if current_price.is_none() {
                        eprintln!("Error: Price for symbol: {} was not found", alert.main_alert.coin_symbol);
                    }

                    let current_price = current_price.unwrap();

                    // TODO: see if we can simplify/condense this - use match which defines a closure?...
                    let mut alert_triggered = false;
                    let m_alert = &alert.main_alert;
                    if m_alert.trigger_type == AlertTriggerType::PriceGreaterThan && current_price > m_alert.trigger_price {
                        alert_triggered = true;
                    }
                    else if m_alert.trigger_type == AlertTriggerType::PriceGreaterThanEqualTo && current_price >= m_alert.trigger_price {
                        alert_triggered = true;
                    }
                    else if m_alert.trigger_type == AlertTriggerType::PriceLessThan && current_price < m_alert.trigger_price {
                        alert_triggered = true;
                    }
                    else if m_alert.trigger_type == AlertTriggerType::PriceLessThanEqualTo && current_price <= m_alert.trigger_price {
                        alert_triggered = true;
                    }
                    
                    if alert_triggered {
                        // TODO:
                        eprintln!("Alert hastriggered for {}, with price: {}", alert.main_alert.coin_symbol,
                                    smart_format(current_price));
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_secs(self.config.alert_config.check_period));
        }
    }
}

fn get_price_for_symbol(symbol: &str, prices: &[CoinPriceItem]) -> Option<f64> {
    for price in prices {
        if price.symbol.to_ascii_lowercase() == symbol {
            return Some(price.current_price);
        }
    }

    return None;
}
