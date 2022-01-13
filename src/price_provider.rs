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

// This is the abstracted version of the info which is displayed and monitored by the core infrastructure.
// Provider implementations should fill in these results.
#[derive(Clone, Debug)]
pub struct CoinPriceItem {
    pub symbol:         String,
    pub name:           String,

    pub current_price:  f64,


    pub high_wm_24h:    f64,
    pub low_wm_24h:     f64,


    pub price_change_24h: f64,
    pub price_change_percentage_24h: f64,
}

pub trait PriceProvider {

    // TODO: return something a bit better, maybe even a struct with a description
    //       of what data fields will be returned by the requests?
    fn configure(&mut self, _config: &Config) -> bool {
        return true;
    }

    fn get_current_prices(& self) -> Vec<CoinPriceItem> {

        return vec![];
    }

}
