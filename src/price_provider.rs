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

use crate::config::{Config};

use std::fmt;

#[derive(Clone, Debug)]
pub struct ConfigDetails {
    // TODO: bitflags might be better here, but given we might reverse how this is
    //       done in the future, keep things simple for the moment...

    pub have_percent_change_1h:     bool,

    pub have_watermarks_24h:        bool,

    pub have_price_change_24h:      bool,
}

impl ConfigDetails {
    pub fn new() -> ConfigDetails {
        ConfigDetails { have_percent_change_1h: false, have_watermarks_24h: true, have_price_change_24h: true }
    }
}

// This is the abstracted version of the info which is displayed and monitored by the core infrastructure.
// Provider implementations should fill in these results.
#[derive(Clone, Debug)]
pub struct CoinPriceItem {
    pub symbol:         String,
    pub name:           String,

    pub current_price:  f64,

    pub watermarks_24h: Option<Watermarks>,

    pub price_change_24h: f64,
    pub percent_change_1h: Option<f64>,
    pub percent_change_24h: f64,
}

#[derive(Clone, Debug)]
pub struct Watermarks {
    pub low:    f64,
    pub high:   f64,
}

impl Watermarks {
    pub fn new(low: f64, high: f64) -> Watermarks {
        Watermarks{ low, high }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum GetDataError {
    ConfigError(String),
    CantConnect(String),
    NoResponse(String),
    InvalidAPIParams(String),
    ParseError(String),
    EmptyResults,
    NotImplemented
}

impl fmt::Display for GetDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            GetDataError::ConfigError(ref err) => write!(f, "Configuration Error: {}", err),
            GetDataError::CantConnect(ref err) => write!(f, "Can't Connect: {}", err),
            GetDataError::NoResponse(ref err) => write!(f, "No Response from API server: {}", err),
            GetDataError::InvalidAPIParams(ref err) => write!(f, "Invalid API params provided: {}", err),
            GetDataError::ParseError(ref err) => write!(f, "Error parsing response: {}", err),
            GetDataError::EmptyResults => write!(f, "Empty results"),
            GetDataError::NotImplemented => write!(f, "Not implemented"),
        }
    }
}

pub trait PriceProvider {

    // TODO: return something a bit better, maybe even a struct with a description
    //       of what data fields will be returned by the requests?
    fn configure(&mut self, _config: &Config) -> Option<ConfigDetails> {
        return None;
    }

    fn get_current_prices(&self) -> Result<Vec<CoinPriceItem>, GetDataError> {
        return Err(GetDataError::NotImplemented);
    }

}
