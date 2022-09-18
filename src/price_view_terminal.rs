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

use crate::config::{Config, DisplayDataViewType};

use crate::price_provider::{PriceProvider, PriceProviderParams, ConfigDetails, CoinPriceItem};

use crate::cli_table_printer::{CLITablePrinter, Alignment};

use crate::formatting_helpers::{smart_format};

//use termion::{color};
use chrono::{Local};

pub struct PriceViewTerminal {

    // our copy of the config...
    config:             Config,

    config_details:     ConfigDetails,

    // what was originally provided to the PriceProvider to configure it.... possibly with deferred modifications (i.e.)
    // wanted_coin_symbols...
    price_provider_params:  PriceProviderParams,
    price_provider:     Box<dyn PriceProvider>,

    table_headings:     Vec<String>,

    table_def:          CLITablePrinter,
}

impl PriceViewTerminal {
    pub fn new(config: &Config, config_details: ConfigDetails, price_provider_params: &PriceProviderParams,
                                 price_provider: Box<dyn PriceProvider>) -> PriceViewTerminal {
        let price_view = PriceViewTerminal{ config: config.clone(),
                                            config_details,
                                            price_provider_params: price_provider_params.clone(),
                                            price_provider,
                                            table_headings: Vec::with_capacity(3),
                                            table_def: CLITablePrinter::new(3) };
        return price_view;
    }

    pub fn run(&mut self) {

        // lazily update the price provider with the symbols we want by reconfiguring it again...
        // Not amazingly happy about this, but I'm less happy with alternatives in this chicken-and-egg situation...
        self.price_provider_params.wanted_coin_symbols = self.config.display_config.wanted_coins.clone();
        let mut_provider = &mut self.price_provider;
        mut_provider.configure(&self.price_provider_params);

        let price_heading = format!("Price ({})", self.config.display_config.fiat_currency.to_ascii_uppercase());
        self.table_headings = vec!["Sym".to_string(), "Name".to_string(), price_heading];

        // configure the master table def based off display/view settings...
        self.table_def.add_titles(&self.table_headings);
        self.table_def.set_alignment_multiple(&[2usize], Alignment::Right);

        // now other optional columns, depending on the display view type wanted
        if self.config.display_config.data_view_type == DisplayDataViewType::MediumData {
            if self.config_details.have_price_change_24h {
                self.table_def.add_column_def("chng 24h", Alignment::Right);
            }
            if self.config_details.have_percent_change_1h {
                self.table_def.add_column_def("% chng 1h", Alignment::Right);
            }
            self.table_def.add_column_def("% chng 24h", Alignment::Right);

            if self.config_details.have_watermarks_24h {
                self.table_def.add_column_def("low 24h", Alignment::Right);
                self.table_def.add_column_def("high 24h", Alignment::Right);
            }
        }

        self.run_display_update_loop();
    }

    fn run_display_update_loop(&self) {

        println!("Fetching prices...");

        loop {
            let results = self.price_provider.get_current_prices();

            print!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1));

            if let Err(err) = results {
                eprintln!("Error getting price results: {}", err);
            }
            else {
                let prices = results.unwrap();

                let local_time = Local::now();
                println!("Cryptmon Price View. Data last updated: {}\n", local_time.format("%d/%m %H:%M:%S"));

                // clone a copy of table def to use..
                // TODO: might want to just reset some contents of it, but then need to think about interior mutablility or something?
                let mut local_table = self.table_def.clone();
                for price in prices {
                    self.add_coin_details_to_table(&mut local_table, &price);
                }

                println!("{}", local_table);
            }

            std::thread::sleep(std::time::Duration::from_secs(self.config.display_config.update_period));
        }
    }

    fn add_coin_details_to_table(&self, table_printer: &mut CLITablePrinter, coin_details: &CoinPriceItem) {
        let current_price = smart_format(coin_details.current_price);

        // not great, but we need the lifetimes' to live to the end, so...
        let change_24hr: String;
        let percent_change_1h: String;
        let percent_change_24hr: String;
        let low_wm_24hr: String;
        let high_wm_24hr: String;

        let mut row_strings: Vec<&str> = vec![&coin_details.symbol, &coin_details.name, &current_price];

        // now other optional columns, depending on the display view type wanted
        if self.config.display_config.data_view_type == DisplayDataViewType::MediumData {
            if self.config_details.have_price_change_24h {
                change_24hr = smart_format(coin_details.price_change_24h);
                row_strings.push(&change_24hr);
            }
            if self.config_details.have_percent_change_1h {
                if let Some(perc_change_1h) = coin_details.percent_change_1h {
                    percent_change_1h = format!("{:.2}%", perc_change_1h);
                    row_strings.push(&percent_change_1h);
                }
            }
            percent_change_24hr = format!("{:.2}%", coin_details.percent_change_24h);
            row_strings.push(&percent_change_24hr);

            if self.config_details.have_watermarks_24h {
                if let Some(ref watermarks) = coin_details.watermarks_24h {
                    low_wm_24hr = smart_format(watermarks.low);
                    high_wm_24hr = smart_format(watermarks.high);

                    row_strings.push(&low_wm_24hr);
                    row_strings.push(&high_wm_24hr);
                }
            }
        }

        table_printer.add_row_strings(&row_strings);
    }
}
