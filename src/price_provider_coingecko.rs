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

use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use crate::price_provider::{PriceProvider, PriceProviderParams, ConfigDetails, GetDataError, CoinPriceItem, Watermarks};

// for results back from CoinGecko's API regarding the list of coins and their IDs
//
// Note: this is public because CoinGecko's API is fast and ideal for this (minimal data), whereas
//       some of the other providers' (i.e. CryptoCompare) return huge amounts of data
//       in their query for the same, so it can take ages to get the results back,
//       so we'll re-use this functionality from CoinGecko within other providers just
//       for the coin list
#[derive(Serialize, Deserialize)]
#[derive(Clone, Debug)]
pub struct CoinListResultItem {
    pub id:         String,
    pub symbol:     String,
    pub name:       String,
}

// for results back from CoinGecko's API regarding the list of prices
#[derive(Serialize, Deserialize)]
#[derive(Clone, Debug)]
struct CoinMarketPriceItem {
    id:                 String,
    symbol:             String,
    name:               String,

    current_price:      f64,

    high_24h:           f64,
    low_24h:            f64,

    price_change_24h:   f64,
    price_change_percentage_24h:    f64,
}

pub struct ProviderCoinGecko {
    params:         PriceProviderParams,

    // list of actual id values to use for the request for prices of the 
    // coins that we actually want (i.e. a subset of the full list)
    ids_wanted:     Vec<String>,
    currency_val:   String,

    // TODO: we could cache this and only update it every few days rather than every startup?
    full_coin_list: Vec<CoinListResultItem>,
}

impl ProviderCoinGecko {
    // TODO: maybe this could be made generic with dyn and put somewhere shared to reduce duplication per-provider?
    pub fn new_from_config(params: &PriceProviderParams) -> Option<(ProviderCoinGecko, ConfigDetails)> {
        let mut provider = ProviderCoinGecko { params: params.clone(), 
                            ids_wanted: Vec::with_capacity(0),
                            currency_val: String::new(), full_coin_list: Vec::with_capacity(0) };
        
        let config_details = provider.configure(params)?;

        return Some((provider, config_details));
    }

    // This is public so other providers can use it in isolation
    // TODO: Use Result for error handling...
    pub fn get_minimal_coin_list() -> Option<Vec<CoinListResultItem>> {
        let coin_list_request = ureq::get(&"https://api.coingecko.com/api/v3/coins/list".to_string());
        let coin_list_resp = coin_list_request.call();        
        if coin_list_resp.is_err() {
            eprintln!("Error calling https://api.coingecko.com/api/v3/coins/list {:?}", coin_list_resp.err());
            return None;
        }

        let coin_list_resp = coin_list_resp.unwrap().into_string().unwrap();

        let full_coin_list: Vec<CoinListResultItem> = serde_json::from_str(&coin_list_resp).unwrap();
        return Some(full_coin_list);
    }
}

impl PriceProvider for ProviderCoinGecko {
    fn configure(&mut self, params: &PriceProviderParams) -> Option<ConfigDetails> {
        // update this in a deferred way, so it can be updated lazily later, rather than
        // just when being created...
        self.params = params.clone();

        let coin_list = ProviderCoinGecko::get_minimal_coin_list()?;
        self.full_coin_list = coin_list;

        // now work out the IDs of the coins we want, based off the symbol
        let mut lookup = BTreeMap::new();

        let mut index = 0usize;
        for coin in &self.full_coin_list {
            // filter out pegged values we don't want, due to symbol collisions..
            // TODO: something smarter than this, but not sure how, given collisions...
            
            // filter item symbols are in lowercase...
            if let Some(val) = self.params.coin_name_ignore_items.get(&coin.symbol.to_ascii_lowercase()) {
                if coin.name.contains(val) {
                    // skip this item
                    index += 1;
                    continue;
                }
            }
            lookup.insert(coin.symbol.to_ascii_uppercase(), index);
            index += 1;
        }

        for coin in &self.params.wanted_coin_symbols {
            if let Some(index) = lookup.get(&coin.to_ascii_uppercase()) {
                let item = &self.full_coin_list[*index];
                self.ids_wanted.push(item.id.clone());
            }
        }

        self.currency_val = params.fiat_currency.to_ascii_lowercase();
        if self.currency_val.is_empty() {
            eprintln!("Error: Fiat Currency value for CoinGecko provider was not specified. Using NZD instead...");
            self.currency_val = "nzd".to_string();
        }

        return Some(ConfigDetails::new());
    }

    fn get_current_prices(&self) -> Result<Vec<CoinPriceItem>, GetDataError> {

        if self.ids_wanted.is_empty() {
            return Err(GetDataError::ConfigError("No currency symbols configured/requested".to_string()));
        }

        let ids_param = self.ids_wanted.join(",");

        let request_url = format!("https://api.coingecko.com/api/v3/coins/markets?vs_currency={}&ids={}",
                                    self.currency_val, ids_param);
        
        let price_results = ureq::get(&request_url).call();
        if price_results.is_err() {
            return Err(GetDataError::CantConnect(format!("Error calling https://api.coingecko.com/api/v3/coins/markets: {:?}", price_results.err())));
        }

        // TODO: error handling!
        let coin_price_resp = price_results.unwrap().into_string().unwrap();

        let coin_price_results = serde_json::from_str::<Vec<CoinMarketPriceItem>>(&coin_price_resp);
        if coin_price_results.is_err() {
            return Err(GetDataError::ParseError(coin_price_results.err().unwrap().to_string()));
        }
        let coin_price_results = coin_price_results.unwrap();

        if coin_price_results.is_empty() {
            return Err(GetDataError::EmptyResults);
        }

        let mut results = Vec::with_capacity(coin_price_results.len());

        for src_res in &coin_price_results {

            let new_val = CoinPriceItem{ symbol: src_res.symbol.to_ascii_uppercase(), name: src_res.name.clone(),
                                        current_price: src_res.current_price,
                                        watermarks_24h: Some(Watermarks::new(src_res.low_24h, src_res.high_24h)),
                                        price_change_24h: src_res.price_change_24h,
                                        percent_change_1h: None,
                                        percent_change_24h: src_res.price_change_percentage_24h };

            results.push(new_val);
        }

        return Ok(results);
    }
}
