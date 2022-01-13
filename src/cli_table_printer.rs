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

use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Alignment {
    Left,
    Right
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum BorderType {
    None,
    Colon,
    Pipe,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ColumnProperties {
    text_alignment:         Alignment,
    next_border_type:       BorderType,
}

impl ColumnProperties {
    pub fn new() -> ColumnProperties {
        return ColumnProperties { text_alignment: Alignment::Left, next_border_type: BorderType::None };
    }
    pub fn new_with_alignment(alignment: Alignment) -> ColumnProperties {
        return ColumnProperties { text_alignment: alignment, next_border_type: BorderType::None };
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct CLITablePrinter {
    num_columns:        usize,
    data_columns:       Vec<Vec<String>>,

    draw_titles:        bool,
    column_titles:      Vec<String>,

    draw_borders:       bool,

    column_properties:  Vec<ColumnProperties>
}



impl CLITablePrinter {
    pub fn new(num_columns: usize) -> CLITablePrinter {
        let new_item = CLITablePrinter { num_columns, data_columns: vec![Vec::new(); num_columns], draw_titles: false,
                                     column_titles: Vec::with_capacity(0), draw_borders: false,
                                     column_properties: vec![ColumnProperties::new(); num_columns] };

        return new_item;
    }

    // pub fn set_alignment(mut self, column: usize, alignment: Alignment) -> Self {
    //     self.column_properties[column].text_alignment = alignment;
    //     return self;
    // }

    pub fn set_alignment_multiple(&mut self, columns: &[usize], alignment: Alignment) {
        for col in columns {
            self.column_properties[*col].text_alignment = alignment;
        }
    }

    pub fn add_titles<T>(&mut self, titles: T)
    where
        T: IntoIterator,
        T::Item: AsRef<str> 
    {
        self.draw_titles = true;
        for title in titles.into_iter() {
            self.column_titles.push(title.as_ref().to_string());
        }
    }

    pub fn add_column_def(&mut self, title: &str, alignment: Alignment) {
        self.column_titles.push(title.to_string());
        self.column_properties.push(ColumnProperties::new_with_alignment(alignment));
        self.data_columns.push(Vec::new());
        self.num_columns += 1;
    }

    // this one starts from the beginning
    pub fn add_row_strings(&mut self, vals: &[&str]) {
        assert!(vals.len() == self.num_columns);

        for (count, string_val) in vals.iter().enumerate() {
            self.data_columns[count].push(string_val.to_string());
        }
    }
/*
    pub fn add_row_strings<T>(&mut self, vals: T)
    where
        T: IntoIterator,
        T::Item: AsRef<str>
    {
        for (count, string_val) in vals.into_iter().enumerate() {
            self.data_columns[count].push(string_val.as_ref().to_string());
        }
    }
*/

    // Note: fmt() is implemented below, and calls this...
    fn get_result(&self) -> String {
        let mut max_column_widths = Vec::with_capacity(self.data_columns.len());

        let row_length = self.data_columns[0].len();

        if self.draw_titles {
            assert!(self.column_titles.len() == self.data_columns.len());
        }

        for (count, column) in self.data_columns.iter().enumerate() {
            assert!(column.len() == row_length);

            let mut max = column.iter().map(|c| c.chars().count()).max().unwrap();
            if self.draw_titles {
                max = std::cmp::max(max, self.column_titles[count].chars().count());
            }

            max_column_widths.push(max);
        }

        let sep_width = 2;

        let mut final_result = String::new();

        if self.draw_titles {
            let mut full_length = 0;
            for (count, title) in self.column_titles.iter().enumerate() {
                let item_length = title.chars().count();
                let padding_required = max_column_widths[count] - item_length;
                let col_properties = &self.column_properties[count];
                let padding_chars = " ".repeat(padding_required);

                if col_properties.text_alignment == Alignment::Left {
                    final_result.push_str(title.as_str());
                    final_result.push_str(&padding_chars);
                }
                else {
                    final_result.push_str(&padding_chars);
                    final_result.push_str(title.as_str());
                }

                full_length += max_column_widths[count];
                if count != (self.num_columns - 1) {
                    for _i in 0..sep_width {
                        final_result.push(' ');
                    }
                    full_length += sep_width;
                }
            }
            final_result.push('\n');
            for _i in 0..full_length {
                final_result.push('-');
            }
            final_result.push('\n');
        }

        for row in 0..row_length {
            for col in 0..self.num_columns {
                let item_string = &self.data_columns[col][row];
                let item_length = item_string.chars().count();

                let padding_required = max_column_widths[col] - item_length;
                let padding_chars = " ".repeat(padding_required);
                
                let col_properties = &self.column_properties[col];

                if col_properties.text_alignment == Alignment::Left {
                    final_result.push_str(item_string.as_str());
                    final_result.push_str(&padding_chars);
                }
                else {
                    final_result.push_str(&padding_chars);
                    final_result.push_str(item_string.as_str());
                }

                if col != (self.num_columns - 1) {
                    // add column sep
                    for _i in 0..sep_width {
                        final_result.push(' ');
                    }
                }
            }
            final_result.push('\n');
        }

        return final_result;
    }
}

impl fmt::Display for CLITablePrinter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_result())
    }
}
