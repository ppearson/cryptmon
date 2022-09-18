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
mod alert_provider;

#[cfg(feature = "smtp")]
mod alert_provider_smtp_mail;

mod alert_provider_pushsafer;
mod alert_provider_simplepush;
mod alert_provider_textbelt;
mod alert_service;

mod price_provider;
mod price_provider_coingecko;
mod price_provider_coinmarketcap;
mod price_provider_cryptocompare;
mod cli_table_printer;
mod price_view_terminal;

mod formatting_helpers;

use config::{Config};

use alert_service::{AlertService};
use price_provider::{PriceProvider, ConfigDetails, PriceProviderParams};
use price_provider_coingecko::{ProviderCoinGecko};
use price_provider_coinmarketcap::{ProviderCoinMarketCap};
use price_provider_cryptocompare::{ProviderCryptoCompare};
use price_view_terminal::PriceViewTerminal;

use std::env;

#[derive(Clone, Debug, Eq, PartialEq)]
enum RunType {
    View,
    Alerts
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::load();
    
    let mut provider: Option<Box<dyn PriceProvider>> = None;
    let mut config_details = ConfigDetails::new();

    let mut run_type = RunType::View;

    if args.len() > 1 {
        let first_arg = &args[1];
        if first_arg == "alerts" {
            run_type = RunType::Alerts;
        }
    }

    // TODO: this whole chicken-and-egg situation with PriceProvider/Config/PriceProviderParams is a mess...
    //       I would really prefer to defer configuring things until later on (i.e. lazily configure as and when)
    //       needed, but doing that in Rust seems quite painful and unidiomatic...
    let data_provider = if run_type == RunType::View { &config.display_config.data_provider } else { &config.alert_config.data_provider };
    let fiat_currency = if run_type == RunType::View { &config.display_config.fiat_currency } else { &config.alert_config.fiat_currency };
    let coin_name_ignore_items = if run_type == RunType::View { &config.display_config.coin_name_ignore_items }
                                                 else { &config.alert_config.coin_name_ignore_items };

    let mut provider_params = PriceProviderParams::new();
    provider_params.fiat_currency = fiat_currency.clone();
    provider_params.coin_name_ignore_items = coin_name_ignore_items.clone();

    // TODO: abstract this away somewhere so it's A: encapsuled, and B: re-useable?
    if data_provider == "coingecko" {
        if let Some((prov, config_dets)) = ProviderCoinGecko::new_from_config(&provider_params) {
            provider = Some(Box::new(prov));
            config_details = config_dets;
        }
    }
    else if data_provider == "coinmarketcap" {
        if let Some((prov, config_dets)) = ProviderCoinMarketCap::new_from_config(&provider_params) {
            provider = Some(Box::new(prov));
            config_details = config_dets;
        }
    }
    else if data_provider == "cryptocompare" {
        if let Some((prov, config_dets)) = ProviderCryptoCompare::new_from_config(&provider_params) {
            provider = Some(Box::new(prov));
            config_details = config_dets;
        }
    }
    else {
        eprintln!("Error: Unknown 'dataProvider' config item specified: {}. Please make sure it is one of the supported price providers.", data_provider);
        return;
    }

    if provider.is_none() {
        eprintln!("Error: Couldn't create required PriceProvider item to obtain coin currency values with. cryptmon will exit.");
        return;
    }

    if run_type == RunType::View {
        let mut price_view = PriceViewTerminal::new(&config, config_details, &provider_params, provider.unwrap());
        price_view.run();
    }
    else if run_type == RunType::Alerts {
        let alert_service = AlertService::new(&config, &provider_params, provider.unwrap());
        if let Some(mut service) = alert_service {
            service.run();
        }
        else {
            eprintln!("Error creating AlertService");
        }
    }    
}
