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
pub enum DisplayDataViewType {
    FullData,
    MediumData, // TODO: This needs a better name!...
    PriceOnly
}

#[derive(Clone, Debug)]
pub struct Config {
    pub data_provider:          String,
    
    pub display_currency:       String,

    pub display_coins:          Vec<String>,

    // key = lowercase symbol, val: string which if found means skip item
    pub coin_name_ignore_items: BTreeMap<String, String>,

    // this is in seconds.
    // Note: The actual config file can have human-readable units, and Config
    //       converts that to seconds when reading the file.
    pub display_update_period:  u32,

    pub display_data_view_type: DisplayDataViewType,
}

impl Config {
    pub fn load() -> Config {
        // set defaults
        let mut config = Config {data_provider: "cryptocompare".to_string(), display_currency: "nzd".to_string(),
                             display_coins: Vec::with_capacity(0), coin_name_ignore_items: BTreeMap::new(),
                             display_update_period: 120, display_data_view_type: DisplayDataViewType::MediumData };

        config.load_config_file();

        return config;
    }

    fn load_config_file(&mut self) {

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
            return;
        }

        let reader = BufReader::new(file.unwrap());

        for line in reader.lines() {
            let line = line.unwrap();

            // ignore empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((item_key, item_val)) = get_key_value_parts(&line) {
                if item_key == "coinNameIgnoreItems" {
                    let pairs = item_val.split(',');
                    for pair in pairs {
                        if let Some((sym, ignore_string)) = pair.split_once('/') {
                            self.coin_name_ignore_items.insert(sym.to_ascii_lowercase(), ignore_string.to_string());
                        }
                    }
                }
                else if item_key == "dataProvider" {
                    self.data_provider = item_val.to_string();
                }
                else if item_key == "displayCoins" {
                    let coin_symbols = item_val.split(',');
                    for symbol in coin_symbols {
                        self.display_coins.push(symbol.to_lowercase());
                    }
                }
                else if item_key == "displayCurrency" {
                    self.display_currency = item_val.to_string();
                }
                else if item_key == "displayDataViewType" {
                    // TODO: error checking, although I hate that with match statements...
                    self.display_data_view_type = match item_val {
                        "priceOnly" => DisplayDataViewType::PriceOnly,
                        "medium" =>    DisplayDataViewType::MediumData,
                        "full" =>      DisplayDataViewType::FullData,
                        _      =>      DisplayDataViewType::MediumData,
                    }
                }
                else if item_key == "displayUpdatePeriod" {
                    if let Some(period_in_secs) = convert_time_period_string_to_seconds(item_val) {
                        self.display_update_period = period_in_secs;
                    }
                    else {
                        // TODO: currently convert_time_period_string_to_seconds() prints, but we probably
                        //       want to do it here, so that we can provide the name of the param item in the error...
                    }
                }
            }
            else {
                eprintln!("Error: malformed line in cryptmon.ini, will be ignored.");
            }
        }
    }
}

fn get_key_value_parts(str_val: &str) -> Option<(&str, &str)> {
    if !str_val.contains(':') {
        return None;
    }

    let mut split = str_val.split(':');
    let (key, val) = (split.next().unwrap(), split.next().unwrap());
    if key.is_empty() || val.is_empty() {
        return None;
    }

    return Some((key.trim(), val.trim()));
}

// TODO: better error handling and reporting, we might need context as well for reporting, so result would be better...
fn convert_time_period_string_to_seconds(str_val: &str) -> Option<u32> {
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
    let parse_result = local_value.parse::<u32>();
    if parse_result.is_err() {
        eprintln!("Error parsing time period value from config.");
        return None;
    }

    let final_time_in_secs = parse_result.unwrap() * mult_to_seconds;

    return Some(final_time_in_secs);
}
