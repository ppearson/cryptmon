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

use crate::price_provider::{PriceProvider, CoinPriceItem};

use crate::cli_table_printer::{CLITablePrinter, Alignment};

use crate::formatting_helpers::{smart_format};

//use termion::{color};
use chrono::{Local};

pub struct PriceViewTerminal {

    // our copy of the config...
    config:             Config,

    price_provider:     Box<dyn PriceProvider>,

    table_headings:     Vec<String>,

    table_def:          CLITablePrinter,
}

impl PriceViewTerminal {
    pub fn new(config: &Config, price_provider: Box<dyn PriceProvider>) -> PriceViewTerminal {
        let price_view = PriceViewTerminal{ config: config.clone(), price_provider,
                                            table_headings: Vec::with_capacity(3),
                                            table_def: CLITablePrinter::new(3) };
        return price_view;
    }

    pub fn run(&mut self) {
        let price_heading = format!("Price ({})", self.config.display_currency.to_ascii_uppercase());
        self.table_headings = vec!["Sym".to_string(), "Name".to_string(), price_heading];

        // configure the master table def based off display/view settings...
        self.table_def.add_titles(&self.table_headings);
        self.table_def.set_alignment_multiple(&[2usize], Alignment::Right);

        // now other optional columns, depending on the display view type wanted
        if self.config.display_data_view_type == DisplayDataViewType::MediumData {
            self.table_def.add_column_def("chng 24h", Alignment::Right);
            self.table_def.add_column_def("% chng 24h", Alignment::Right);

            self.table_def.add_column_def("low 24h", Alignment::Right);
            self.table_def.add_column_def("high 24h", Alignment::Right);
        }

        self.run_display_update_loop();
    }

    fn run_display_update_loop(&self) {
        loop {
            let results = self.price_provider.get_current_prices();

            print!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1));

            if results.is_empty() {
                println!("Error: No data returned!");
            }
            else {
                let local_time = Local::now();
                println!("Cryptmon Price View. Data last updated: {}\n", local_time.format("%d/%m %H:%M:%S"));

                // clone a copy of table def to use..
                // TODO: might want to just reset some contents of it, but then need to think about interior mutablility or something?
                let mut local_table = self.table_def.clone();
                for price in results {
                    self.add_coin_details_to_table(&mut local_table, &price);
                }

                println!("{}", local_table);
            }

            std::thread::sleep(std::time::Duration::from_secs(self.config.display_update_period as u64));
        }
    }

    fn add_coin_details_to_table(&self, table_printer: &mut CLITablePrinter, coin_details: &CoinPriceItem) {
        let current_price = smart_format(coin_details.current_price);

        // not great, but we need the lifetimes' to live to the end, so...
        let change_24hr: String;
        let percent_change_24hr: String;
        let low_wm_24hr: String;
        let high_wm_24hr: String;

        let mut row_strings: Vec<&str> = vec![&coin_details.symbol, &coin_details.name, &current_price];

        // now other optional columns, depending on the display view type wanted
        if self.config.display_data_view_type == DisplayDataViewType::MediumData {
            change_24hr = smart_format(coin_details.price_change_24h);
            percent_change_24hr = format!("{:.2}%", coin_details.price_change_percentage_24h);

            low_wm_24hr = smart_format(coin_details.low_wm_24h);
            high_wm_24hr = smart_format(coin_details.high_wm_24h);

            row_strings.push(&change_24hr);
            row_strings.push(&percent_change_24hr);
            row_strings.push(&low_wm_24hr);
            row_strings.push(&high_wm_24hr);
        }

        table_printer.add_row_strings(&row_strings);
    }
}
