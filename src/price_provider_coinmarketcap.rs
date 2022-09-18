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

use crate::price_provider::{PriceProvider, PriceProviderParams, ConfigDetails, GetDataError, CoinPriceItem};

#[derive(Serialize, Deserialize)]
#[derive(Clone, Debug)]
struct CoinMarketCapQuoteResults {
    data:           BTreeMap<String, CoinMarketCapDataQuote>,
}

#[derive(Serialize, Deserialize)]
#[derive(Clone, Debug)]
struct CoinMarketCapDataQuote {
    id:                     u64,
    name:                   String,
    symbol:                 String,
    slug:                   String,

    is_active:              u32,
    
    quote:                  BTreeMap<String, CoinMarketCapPriceQuoteConversion>,
}

// this item represents conversions into fiat currencies
#[derive(Serialize, Deserialize)]
#[derive(Clone, Debug)]
struct CoinMarketCapPriceQuoteConversion {
    price:                  f64,
    
    volume_24h:             f64,

    percent_change_1h:      f64,
    percent_change_24h:     f64,
    percent_change_7d:      f64,
    percent_change_30d:     f64,
}

pub struct ProviderCoinMarketCap {
    params:         PriceProviderParams,
    api_key:        String,
}

impl ProviderCoinMarketCap {
    // TODO: maybe this could be made generic with dyn and put somewhere shared to reduce duplication per-provider?
    pub fn new_from_config(params: &PriceProviderParams) -> Option<(ProviderCoinMarketCap, ConfigDetails)> {
        let mut provider = ProviderCoinMarketCap { params: params.clone(), 
                            api_key: String::new() };
        
        let config_details = provider.configure(params)?;

        return Some((provider, config_details));
    }
}

impl PriceProvider for ProviderCoinMarketCap {
    fn configure(&mut self, params: &PriceProviderParams) -> Option<ConfigDetails> {
        // update this in a deferred way, so it can be updated lazily later, rather than
        // just when being created...
        self.params = params.clone();

        if let Some(api_key) = std::env::var_os("COINMARKETCAP_API_KEY") {
            if !api_key.is_empty() {
                self.api_key = api_key.to_str().unwrap().to_string();
                let mut config_details = ConfigDetails::new();
                config_details.have_percent_change_1h = true;
                config_details.have_price_change_24h = false;
                config_details.have_watermarks_24h = false;
                return Some(config_details);
            }
        }

        eprintln!("Error: ProviderCoinMarketCap was not configured correctly. Make sure the $COINMARKETCAP_API_KEY env variable is set.");
        return None;
    }

    fn get_current_prices(&self) -> Result<Vec<CoinPriceItem>, GetDataError> {
        if self.params.wanted_coin_symbols.is_empty() {
            return Err(GetDataError::ConfigError("No coin currency symbols configured/requested".to_string()));
        }

        let symbol_param = self.params.wanted_coin_symbols.join(",");
        let currency = self.params.fiat_currency.clone();

        let request_url = format!("https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?convert={}&symbol={}",
                                currency, symbol_param);
        
        // X-CMC_PRO_API_KEY
        let price_results = ureq::get(&request_url)
                .set("X-CMC_PRO_API_KEY", &self.api_key)
                .call();
        if price_results.is_err() {
            return Err(GetDataError::CantConnect(format!("Error calling https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest: {:?}", price_results.err())));
        }

        // TODO: error handling!
        let coin_price_resp = price_results.unwrap().into_string().unwrap();
 
        let coin_price_results = serde_json::from_str::<CoinMarketCapQuoteResults>(&coin_price_resp);
        if coin_price_results.is_err() {
            return Err(GetDataError::ParseError(coin_price_results.err().unwrap().to_string()));
        }
        let coin_price_results = coin_price_results.unwrap();
 
        if coin_price_results.data.is_empty() {
            return Err(GetDataError::EmptyResults);
        }

        let mut results = Vec::with_capacity(coin_price_results.data.len());

        for coin_symbol in &self.params.wanted_coin_symbols {
            if let Some(coin_item) = coin_price_results.data.get(&coin_symbol.to_ascii_uppercase()) {
                if let Some(currency_item) = coin_item.quote.get(&self.params.fiat_currency.to_ascii_uppercase()) {

                    let new_val = CoinPriceItem{ symbol: coin_item.symbol.to_ascii_uppercase(), name: coin_item.name.clone(),
                        current_price: currency_item.price,
                        watermarks_24h: None,
                        price_change_24h: 0.0,
                        percent_change_1h: Some(currency_item.percent_change_1h),
                        percent_change_24h: currency_item.percent_change_24h };

                    results.push(new_val);
                }
            }
        }

        return Ok(results);
    }
}
