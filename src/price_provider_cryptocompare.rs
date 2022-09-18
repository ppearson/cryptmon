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

use serde_json::{Value};
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::price_provider::Watermarks;
use crate::price_provider::{PriceProvider, PriceProviderParams, ConfigDetails, GetDataError, CoinPriceItem};
use crate::price_provider_coingecko;

// Note: the https://min-api.cryptocompare.com/data/pricemultifull API seems to very often
//       (but not always!!?) return incorrect (out of date?) price results (and other values, like min/max)
//       but the values do update very regularly, so not really sure what's going on...

#[derive(Serialize, Deserialize)]
#[derive(Clone, Debug)]
// Note: UPPERCASE doesn't get rid of the underscore like camelCase does, so we have to use rename attrib on each
//       of them for the moment...
// https://github.com/serde-rs/serde/issues/2153
//#[serde(rename_all = "UPPERCASE")]
struct CoinPriceResultItem {
    #[serde(rename = "FROMSYMBOL")] 
    from_symbol:                String,
    #[serde(rename = "TOSYMBOL")] 
    to_symbol:                  String,

    #[serde(rename = "PRICE")] 
    price:                      f64,

    #[serde(rename = "HIGH24HOUR")] 
    high_24_hour:               f64,
    #[serde(rename = "LOW24HOUR")] 
    low_24_hour:                f64,

    #[serde(rename = "HIGHHOUR")]
    high_hour:                  f64,
    #[serde(rename = "LOWHOUR")]
    low_hour:                   f64,

    #[serde(rename = "CHANGE24HOUR")]
    change_24_hour:             f64,
    #[serde(rename = "CHANGEPCT24HOUR")] 
    change_pct_24_hour:         f64,

    #[serde(rename = "CHANGEHOUR")]
    change_hour:                f64,
    #[serde(rename = "CHANGEPCTHOUR")]
    change_pct_hour:            f64,
}

#[derive(Serialize, Deserialize)]
#[derive(Clone, Debug)]
struct CoinListResults {
    #[serde(rename = "Data")] 
    data:   BTreeMap<String, CoinDefItem>,
}

#[derive(Serialize, Deserialize)]
#[derive(Clone, Debug)]
#[serde(rename_all = "PascalCase")]
struct CoinDefItem {
    symbol:         String,
    coin_name:      String,
}

pub struct ProviderCryptoCompare {
    params:         PriceProviderParams,

    symbols_wanted: Vec<String>,
    currency_val:   String,

    // lookup of symbol to full name...
    // TODO: we could cache this and only update it every few days rather than every startup?
    // <symbol, full_name>
    name_lookup:    BTreeMap<String, String>,
}

// list of coins and names
// 

impl ProviderCryptoCompare {
    // TODO: maybe this could be made generic with dyn and put somewhere shared to reduce duplication per-provider?
    pub fn new_from_config(params: &PriceProviderParams) -> Option<(ProviderCryptoCompare, ConfigDetails)> {
        let mut provider = ProviderCryptoCompare { params: params.clone(), 
                            symbols_wanted: Vec::with_capacity(0),
                            currency_val: String::new(),
                            name_lookup: BTreeMap::new() };
        
        let config_details = provider.configure(params)?;

        return Some((provider, config_details));
    }

    // this one is a lot faster (minimal data), but uses another provider's API
    // Note: wanted_coins is uppercase for the symbols.
    fn build_coin_name_lookup_coingecko(&mut self, wanted_coins: &BTreeSet<String>) -> bool {
        let coin_list = price_provider_coingecko::ProviderCoinGecko::get_minimal_coin_list();
        if let Ok(coins) = coin_list {
            for coin in coins {
                // the coin list symbols from CoinGecko are in lowercase...
                let uppercase_symbol = coin.symbol.to_uppercase();

                if wanted_coins.contains(&uppercase_symbol) {
                    // Note: The coin price data from CryptoCompare currently has the symbols in uppercase,
                    //       so to save compute during conversion within the price lookup, convert
                    //       the subset we want to uppercase

                    // filter out pegged values we don't want, due to symbol collisions..
                    // TODO: something smarter than this, but not sure how, given collisions...
                    
                    // filter item symbols are in lowercase...
                    if let Some(val) = self.params.coin_name_ignore_items.get(&coin.symbol) {
                        if coin.name.contains(val) {
                            // skip this item
                            continue;
                        }
                    }

                    self.name_lookup.insert(uppercase_symbol, coin.name.clone());
                }
            }

            return true;
        }
        else {
            eprintln!("Error: Couldn't retrieve minimal coin list lookup from CoinGecko provider. Full error: {}", coin_list.err().unwrap());

            return false;
        }
    }

    // this one uses our provider's API, but is very slow as it returns huge amounts of data
    // Note: wanted_coins is uppercase for the symbols.
    #[allow(dead_code)]
    fn build_coin_name_lookup_cryptocompare(&mut self, wanted_coins: &BTreeSet<String>) -> bool {
        let coin_list_request = ureq::get("https://min-api.cryptocompare.com/data/all/coinlist");
        let coin_list_resp = coin_list_request.call();        
        if coin_list_resp.is_err() {
            eprintln!("Error calling https://min-api.cryptocompare.com/data/all/coinlist {:?}", coin_list_resp.err());
            return false;
        }

        let coin_list_resp = coin_list_resp.unwrap().into_string().unwrap();

        // TODO: just brute-force it for the moment, given we're only doing it once at startup...
        let full_coin_list: CoinListResults = serde_json::from_str(&coin_list_resp).unwrap();
        for coin_item in &full_coin_list.data {
            if wanted_coins.contains(coin_item.0) {
                // it's one we want, so cache it in the name lookup
                // Note: The coin list data currently has the symbols in uppercase,
                //       so to save compute during conversion, convert the subset we want to uppercase
                self.name_lookup.insert(coin_item.1.symbol.clone(), coin_item.1.coin_name.clone());
            } 
        }

        return true;
    }
}

impl PriceProvider for ProviderCryptoCompare {
    fn configure(&mut self, params: &PriceProviderParams) -> Option<ConfigDetails> {

        // update this in a deferred way, so it can be updated lazily later, rather than
        // just when being created...
        self.params = params.clone();

        // for name lookup later...
        let mut wanted_coins = BTreeSet::new();
        
        for coin in &self.params.wanted_coin_symbols {
            self.symbols_wanted.push(coin.to_ascii_lowercase());

            // Note: The coin list data currently has the symbols in uppercase,
            //       so to save compute during conversion, convert the subset we want to uppercase
            wanted_coins.insert(coin.to_ascii_uppercase());
        }

        self.currency_val = params.fiat_currency.to_ascii_lowercase();
        if self.currency_val.is_empty() {
            eprintln!("Error: Currency value for CryptoCompare provider was not specified. Using NZD instead...");
            self.currency_val = "nzd".to_string();
        }

        // use the CoinGecko one as it's much faster...
        self.build_coin_name_lookup_coingecko(&wanted_coins);

        return Some(ConfigDetails::new());
    }

    fn get_current_prices(&self) -> Result<Vec<CoinPriceItem>, GetDataError> {
        if self.symbols_wanted.is_empty() {
            return Err(GetDataError::ConfigError("No currency symbols configured/requested".to_string()));
        }

        let fsyms_param = self.symbols_wanted.join(",");

        let request_url = format!("https://min-api.cryptocompare.com/data/pricemultifull?fsyms={}&tsyms={}",
                                    fsyms_param, self.currency_val);
        
        let price_results = ureq::get(&request_url).call();
        if price_results.is_err() {
            return Err(GetDataError::CantConnect(format!("Error calling https://min-api.cryptocompare.com/data/pricemultifull: {:?}", price_results.err())));
        }

        // TODO: error handling!
        let coin_price_resp = price_results.unwrap().into_string().unwrap();

        let parsed_response = serde_json::from_str::<Value>(&coin_price_resp);
        if parsed_response.is_err() {
            return Err(GetDataError::ParseError(parsed_response.err().unwrap().to_string()));
        }

        let parsed_value_map = parsed_response.ok().unwrap();

        let mut results = Vec::with_capacity(0);

        // TODO: the below is pretty disgusting, but I couldn't get nested BTreeMap items
        //       to work as expected with Serde with structs, so I'm doing it manually, as
        //       that at least works.
        // check it's an array object and other stuff (i.e. check the json is expected)
        if parsed_value_map.is_object() {
            let value_as_object = parsed_value_map.as_object().unwrap();
            // we only expect 1 actual instance value...
            let raw_map = value_as_object.get("RAW");

            if let Some(coin) = raw_map {
                let coin_map = coin.as_object().unwrap();

                // we should only have one currency, so we can get away with this for the moment...
                for currency in coin_map {
                    let currency_map = currency.1.as_object().unwrap();

                    for item in currency_map {
                        // println!("{:?}", item.1);

                        let result_item = CoinPriceResultItem::deserialize(item.1).unwrap();

                        let coin_symbol = result_item.from_symbol.to_ascii_uppercase();

                        let coin_name = match self.name_lookup.get(&coin_symbol) {
                            Some(name) => name.clone(),
                            _ =>          "Unknown".to_string()
                        };

                        let new_val = CoinPriceItem{ symbol: coin_symbol, name: coin_name,
                                        current_price: result_item.price,
                                        watermarks_24h: Some(Watermarks::new(result_item.low_24_hour, result_item.high_24_hour)),
                                        price_change_24h: result_item.change_24_hour,
                                        percent_change_1h: None,
                                        percent_change_24h: result_item.change_pct_24_hour };
                        
                        results.push(new_val);
                    }
                }
            }
        }

        return Ok(results);
    }
}
