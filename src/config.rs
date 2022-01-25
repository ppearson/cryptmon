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

use std::io::{BufRead, BufReader};

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigSubType {
    None,
    Display,
    Alerts
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayDataViewType {
    PriceOnly,
    MediumData, // TODO: This needs a better name!...
    FullData,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub display_config:     DisplayConfig,

    pub alert_config:       AlertConfig,
}

#[derive(Clone, Debug)]
pub struct DisplayConfig {
    pub data_provider:          String,
    
    pub fiat_currency:          String,

    pub wanted_coins:           Vec<String>,

    // key = lowercase symbol, val: string which if found means skip item
    pub coin_name_ignore_items: BTreeMap<String, String>,

    // this is in seconds.
    // Note: The actual config file can have human-readable units, and Config
    //       converts that to seconds when reading the file.
    pub update_period:          u64,

    pub data_view_type:         DisplayDataViewType,
}

#[derive(Clone, Debug)]
pub struct AlertConfig {
    // Note: for the moment duplicate, as we may want to split them in the future...
    pub data_provider:          String,

    pub fiat_currency:          String,

    // key = lowercase symbol, val: string which if found means skip item
    pub coin_name_ignore_items: BTreeMap<String, String>,

    // time in seconds between each check of alert rules.
    // Note: The actual config file can have human-readable units, and Config
    //       converts that to seconds when reading the file.
    pub check_period:           u64,

    // time in seconds to not alert again after an initial alert, per alert...
    pub general_sleep_period:    u64,

    // trip alert sleeps on a per-alert basis based off hight/low watermark values...
    pub watermark_trip_sleep_enabled: bool,
    pub watermark_trip_sleep_period: u64,

    pub alert_provider_configs: BTreeMap<String, AlertProviderConfig>,

    // for the moment, we'll do this, and defer actual processing of config strings
    // to AlertItems for AlertService to handle, so that that module and this module aren't
    // completely tightly-coupled together, although we may want to revisit this...
    // Note: this has already got a little messy, and the below only exist if the state
    // wasn't extracted first for 'alert_provider_configs' above...
    pub alert_config_strings:   Vec<String>,
}

// I don't *really* like having this here, as it doesn't seem *completely* right, but I don't
// really think there's any perfect place for it, and I think it makes sense to have it here
// given there's specific Config logic which extracts the state for AlertProviders to use...
#[derive(Clone, Debug)]
pub struct AlertProviderConfig {
    pub name:       String,
    pub enabled:    bool,

    pub params:     BTreeMap<String, String>,
}

impl AlertProviderConfig {
    pub fn new(name: &str, enabled: bool) -> AlertProviderConfig {
        return AlertProviderConfig { name: name.to_string(), enabled, params: BTreeMap::new() };
    }

    pub fn get_param_as_string(&self, name: &str) -> Option<String> {
        if let Some(val) = self.params.get(name) {
            return Some(val.to_string());
        }

        return None;
    }
}

impl Config {
    pub fn load() -> Config {
        // set defaults
        let display_config = DisplayConfig {data_provider: "cryptocompare".to_string(), fiat_currency: "nzd".to_string(),
                             wanted_coins: Vec::with_capacity(0), coin_name_ignore_items: BTreeMap::new(),
                             update_period: 120, data_view_type: DisplayDataViewType::MediumData };
        
        let alert_config = AlertConfig {data_provider: "cryptocompare".to_string(), fiat_currency: "nzd".to_string(),
                                    coin_name_ignore_items: BTreeMap::new(), check_period: 120,
                                    general_sleep_period: convert_time_period_string_to_seconds("1h").unwrap(),
                                    watermark_trip_sleep_enabled: false,
                                    watermark_trip_sleep_period: convert_time_period_string_to_seconds("6h").unwrap(),
                                    alert_provider_configs: BTreeMap::new(),
                                    alert_config_strings: Vec::with_capacity(0) };
        
        let mut config = Config { display_config, alert_config };

        if !config.load_config_file() {
            // we didn't find a config file, so add some currency symbols as the default so we at least load something by default...
            config.display_config.wanted_coins.push("BTC".to_string());
            config.display_config.wanted_coins.push("ETH".to_string());
            config.display_config.wanted_coins.push("BTC".to_string());
            config.display_config.wanted_coins.push("LTC".to_string());
        }

        return config;
    }

    fn load_config_file(&mut self) -> bool {

        // TODO: would be nice to condense this a bit...
        let mut config_path = String::new();
        // first try env varible...
        if let Some(conf_path_env_var) = std::env::var_os("CRYPTMON_CONFIG_PATH") {
            if !conf_path_env_var.is_empty() {
                config_path = conf_path_env_var.to_str().unwrap().to_string();
            }
        }

        if config_path.is_empty() {
            // then try common places..

            // TODO: should probably only do this for Linux, but might be useful for others as well...

            // TODO: $XDG_CONFIG_HOME ?

            if let Some(home_env_var) = std::env::var_os("HOME") {
                if !home_env_var.is_empty() {
                    let test_config_path = format!("{}/.config/cryptmon.ini", home_env_var.to_str().unwrap());
                    if std::path::Path::new(&test_config_path).exists() {
                        config_path = test_config_path;
                    } 
                }
            }
        }

        // if it's STILL empty, at least for the moment during dev, do this...
        if config_path.is_empty() {
            // for the moment...
            #[cfg(target_os = "macos")]
            let temp_config_path = "/Users/peter/cryptmon.ini";
            #[cfg(target_os = "linux")]
            let temp_config_path = "/home/peter/cryptmon.ini";

            config_path = temp_config_path.to_string();
        }

        let file = std::fs::File::open(config_path);
        if file.is_err() {
            eprintln!("Warning: Can't find a cryptmon.ini file for config, so using default configuration...");
            return false;
        }

        let reader = BufReader::new(file.unwrap());

        for line in reader.lines() {
            let line = line.unwrap();

            // ignore empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // TODO: we can likely condense quite a bit of this...
            if let Some((sub_type, item_key, item_val)) = get_key_value_parts(&line) {
                if item_key == "coinNameIgnoreItems" {
                    let mut temp_items: BTreeMap<String, String> = BTreeMap::new();
                    let pairs = item_val.split(',');
                    for pair in pairs {
                        if let Some((sym, ignore_string)) = pair.split_once('/') {
                            temp_items.insert(sym.to_ascii_lowercase(), ignore_string.to_string());
                        }
                    }
                    if sub_type == ConfigSubType::None || sub_type == ConfigSubType::Display {
                        self.display_config.coin_name_ignore_items.append(&mut temp_items.clone());
                    }
                    if sub_type == ConfigSubType::None || sub_type == ConfigSubType::Alerts {
                        self.alert_config.coin_name_ignore_items.append(&mut temp_items.clone());
                    }
                }
                else if item_key == "dataProvider" {
                    if sub_type == ConfigSubType::None || sub_type == ConfigSubType::Display {
                        self.display_config.data_provider = item_val.to_string();
                    }
                    if sub_type == ConfigSubType::None || sub_type == ConfigSubType::Alerts {
                        self.alert_config.data_provider = item_val.to_string();
                    }
                }
                else if sub_type == ConfigSubType::Display && item_key == "wantedCoins" {
                    let coin_symbols = item_val.split(',');
                    for symbol in coin_symbols {
                        self.display_config.wanted_coins.push(symbol.to_lowercase());
                    }
                }
                else if item_key == "fiatCurrency" {
                    if sub_type == ConfigSubType::None || sub_type == ConfigSubType::Display {
                        self.display_config.fiat_currency = item_val.to_string();
                    }
                    if sub_type == ConfigSubType::None || sub_type == ConfigSubType::Alerts {
                        self.alert_config.fiat_currency = item_val.to_string();
                    }
                }
                else if sub_type == ConfigSubType::Display && item_key == "displayDataViewType" {
                    // TODO: error checking, although I hate that with match statements...
                    self.display_config.data_view_type = match item_val {
                        "priceOnly" => DisplayDataViewType::PriceOnly,
                        "medium" =>    DisplayDataViewType::MediumData,
                        "full" =>      DisplayDataViewType::FullData,
                        _      =>      DisplayDataViewType::MediumData,
                    }
                }
                else if sub_type == ConfigSubType::Display && item_key == "updatePeriod" {
                    if let Some(period_in_secs) = convert_time_period_string_to_seconds(item_val) {
                        self.display_config.update_period = period_in_secs;
                    }
                    else {
                        // TODO: currently convert_time_period_string_to_seconds() prints, but we probably
                        //       want to do it here, so that we can provide the name of the param item in the error...
                    }
                }
                else if sub_type == ConfigSubType::Alerts && item_key == "checkPeriod" {
                    if let Some(period_in_secs) = convert_time_period_string_to_seconds(item_val) {
                        self.alert_config.check_period = period_in_secs;
                    }
                    else {
                        // TODO: currently convert_time_period_string_to_seconds() prints, but we probably
                        //       want to do it here, so that we can provide the name of the param item in the error...
                    }
                }
                else if sub_type == ConfigSubType::Alerts && item_key == "generalSleepPeriod" {
                    if let Some(period_in_secs) = convert_time_period_string_to_seconds(item_val) {
                        self.alert_config.general_sleep_period = period_in_secs;
                    }
                    else {
                        // TODO: currently convert_time_period_string_to_seconds() prints, but we probably
                        //       want to do it here, so that we can provide the name of the param item in the error...
                    }
                }
                else if sub_type == ConfigSubType::Alerts && item_key == "watermarkTripSleepEnabled" {
                    self.alert_config.watermark_trip_sleep_enabled = item_val == "true" || item_val == "1";
                }
                else if sub_type == ConfigSubType::Alerts && item_key == "watermarkTripSleepPeriod" {
                    if let Some(period_in_secs) = convert_time_period_string_to_seconds(item_val) {
                        self.alert_config.watermark_trip_sleep_period = period_in_secs;
                    }
                    else {
                        // TODO: currently convert_time_period_string_to_seconds() prints, but we probably
                        //       want to do it here, so that we can provide the name of the param item in the error...
                    }
                }
                else if sub_type == ConfigSubType::Alerts && item_key.starts_with("provider.") {
                    if let Some(definition_key) = item_key.strip_prefix("provider.") {
                        if let Some(provider_name_end) = definition_key.find('.') {
                            let provider_name = &definition_key[..provider_name_end];
                            let param_name = &definition_key[provider_name_end + 1..];

                            let mut prov_config = self.alert_config.alert_provider_configs.get_mut(provider_name);
                            if prov_config.is_none() {
                                // it doesn't exist yet, so create it in-place
                                self.alert_config.alert_provider_configs.insert(provider_name.to_string(), AlertProviderConfig::new(provider_name, false));
                                prov_config = self.alert_config.alert_provider_configs.get_mut(provider_name);
                            }
                            let prov_config = prov_config.unwrap();
                            
                            if param_name == "enabled" {
                                prov_config.enabled = item_val == "true" || item_val == "1";
                            }
                            else {
                                // it's a generic key/value param, so add it to the list
                                prov_config.params.insert(param_name.to_string(), item_val.to_string());
                            }

                            // continue, so we skip over the error message as we've handled it
                            continue;
                        }
                    }

                    eprintln!("Error processing alert provider config: {} - {}", item_key, item_val);
                }
                else if sub_type == ConfigSubType::Alerts && item_key == "newAlert" {
                    self.alert_config.alert_config_strings.push(item_val.to_string());
                }
            }
            else {
                eprintln!("Error: malformed line in cryptmon.ini, will be ignored.");
            }
        }

        return true;
    }
}

fn get_key_value_parts(str_val: &str) -> Option<(ConfigSubType, &str, &str)> {
    let split = str_val.split_once(':')?;
    let (mut key, val) = split;
    if key.is_empty() || val.is_empty() {
        return None;
    }

    let mut ctype = ConfigSubType::None;
    if let Some((left, right)) = key.split_once('.') {
        if left == "display" {
            ctype = ConfigSubType::Display;
        }
        else if left == "alerts" {
            ctype = ConfigSubType::Alerts;
        }
        key = right;
    }

    return Some((ctype, key.trim(), val.trim()));
}

// TODO: better error handling and reporting, we might need context as well for reporting, so result would be better...
fn convert_time_period_string_to_seconds(str_val: &str) -> Option<u64> {
    // TODO: there's probably a better way of doing this...
    let mut local_value = str_val.to_string();
    let last_char = str_val.chars().last().unwrap();
    let value_was_unitless = !last_char.is_alphabetic();
    
    let mult_to_seconds;
    match last_char {
        's' => { mult_to_seconds = 1; }
        'm' => { mult_to_seconds = 60; }
        'h' => { mult_to_seconds = 60 * 60; }
        _   => { mult_to_seconds = 60;  }
    }

    if !value_was_unitless {
        local_value.pop();
    }
    let parse_result = local_value.parse::<u64>();
    if parse_result.is_err() {
        eprintln!("Error parsing time period value from config.");
        return None;
    }

    let final_time_in_secs = parse_result.unwrap() * mult_to_seconds;

    return Some(final_time_in_secs);
}


