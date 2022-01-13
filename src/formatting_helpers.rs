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

// attempt to format to a precision based off magnitude of value, i.e. smaller values have more significant places
pub fn smart_format(val: f64) -> String {
    let abs_value = val.abs();
    let mut precision = 2usize;
    
    // TODO: this needs more work to be more robust, we can probably generalise it more as well
    if abs_value >= 10.0 {
        // nothing...
    }
    else if abs_value >= 1.0 {
        precision += 1;
    }
    else {
        precision += 2;
    }
    let mut format_str = format!("{:.prec$}", val, prec = precision);

    // add comma thousands sep if needed
    if abs_value >= 1000.0 {
        let decimal_place = format_str.find('.');
        let thousands_pos = decimal_place.unwrap() - 3;
        format_str.insert(thousands_pos, ',');
    }

    // TODO: maybe strip off more than one trailing '0's after the decimal?

    return format_str;
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_format_01() {
        
        assert_eq!(smart_format(64000.0), "64,000.00");
        assert_eq!(smart_format(4300.4), "4,300.40");
        assert_eq!(smart_format(128.52), "128.52");

        assert_eq!(smart_format(-64000.0), "-64,000.00");
        assert_eq!(smart_format(-4140.2), "-4,140.20");
        assert_eq!(smart_format(-128.52), "-128.52");    
    }
}