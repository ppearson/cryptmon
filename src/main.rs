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

mod config;
mod price_provider;
mod price_provider_coingecko;
mod price_provider_cryptocompare;
mod cli_table_printer;
mod formatting_helpers;
mod price_view_terminal;

use config::Config;

use price_provider::{PriceProvider};
use price_provider_coingecko::{ProviderCoinGecko};
use price_provider_cryptocompare::{ProviderCryptoCompare};
use price_view_terminal::PriceViewTerminal;

fn main() {
    let config = Config::load();
    
    let provider: Box<dyn PriceProvider>;
    // TODO: abstract this away somewhere so it's A: encapsuled, and B: re-useable?
    if config.data_provider == "coingecko" {
        provider = Box::new(ProviderCoinGecko::new_from_config(&config));
    }
    else if config.data_provider == "cryptocompare" {
        provider = Box::new(ProviderCryptoCompare::new_from_config(&config));
    }
    else {
        eprintln!("Error: Unknown 'dataProvider' config item specified.");
        return;
    }

    let mut price_view = PriceViewTerminal::new(&config, provider);
    price_view.run();
}
