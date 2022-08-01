use colored::Colorize;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn construct_currency_vector() -> Vec<String> {
    let file = File::open("codes_unique_sorted.txt").expect("Error reading file");
    let reader = BufReader::new(file);

    // Some turbofish magic, I don't know
    reader.lines().collect::<Result<_, _>>().unwrap()
}

mod user_input_processing {
    use regex::Regex;
    use std::io;

    fn get_user_input() -> String {
        println!("Enter currency pair:");
        let mut input = String::new();

        io::stdin()
            .read_line(&mut input)
            .expect("Couldn't read line");

        input.trim().to_uppercase()
    }

    fn is_input_in_valid_format(user_input: String) -> Option<(String, String)> {
        let re = Regex::new(r"^(?P<cur1>[a-zA-Z]{3}) (?P<cur2>[a-zA-Z]{3})$")
            .expect("Couldn't construct regex");

        let captures = re.captures(&user_input)?;
        Some((captures["cur1"].to_string(), captures["cur2"].to_string()))
    }

    /// A wrapper function around the above `get_user_input`() and
    /// `dissect_user_input`() functions. Guaranteed to return a valid
    /// currency pair (if one of the functions it calls doesn't panic).
    pub fn get_valid_currency_codes() -> (String, String) {
        let currency_vector = crate::construct_currency_vector();
        loop {
            let user_input = get_user_input();
            if let Some(currencies) = is_input_in_valid_format(user_input) {
                if !currency_vector.contains(&currencies.0)
                    || !currency_vector.contains(&currencies.1)
                {
                    println!("Invalid currency code");
                    continue;
                }
                return currencies;
            } else {
                println!("Invalid input format");
                continue;
            }
        }
    }
}

mod price_operations {
    use serde_json::{self, Value};
    use std::collections::HashMap;
    use std::{env, error::Error};

    // Returns raw response from the API
    pub fn get_exchange_rate_raw(
        cur1: String,
        cur2: String,
        yesterday_date: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        let api_key =
            env::var("CURRENCY_API_KEY").expect("CURRENCY_API_KEY environment variable not set");

        let url_today = format!(
            "https://free.currconv.com/api/v7/convert?q={}_{}&compact=ultra&apiKey={}",
            cur1, cur2, api_key
        );

        let url_yesterday = format!(
            "https://free.currconv.com/api/v7/convert?q={}_{}&compact=ultra&date={}&apiKey={}",
            cur1, cur2, yesterday_date, api_key,
        );

        let resp_today = reqwest::blocking::get(url_today)?.text()?;
        let resp_yesterday = reqwest::blocking::get(url_yesterday)?.text()?;

        return Ok((resp_today, resp_yesterday));
    }

    // TODO don't unwrap
    pub fn get_prices_from_api_response(
        exchange_rates_raw: (String, String),
        yesterday_date: &str,
    ) -> (f32, f32) {
        // Convert the raw strings into hashmaps
        let today_price_map: HashMap<String, Value> =
            serde_json::from_str(&exchange_rates_raw.0).expect("JSON was not well-formatted");
        let yesterday_price_map: HashMap<String, Value> =
            serde_json::from_str(&exchange_rates_raw.1).expect("JSON was not well-formatted");

        // Get the numerical values of the exchange rates
        let today_price = today_price_map.get("USD_CHF").unwrap().to_string();
        let yesterday_price = yesterday_price_map
            .get("USD_CHF")
            .unwrap()
            .get(yesterday_date)
            .unwrap()
            .to_string();

        // Convert the string values to floats
        let today_price = today_price.parse::<f32>().unwrap();
        let yesterday_price = yesterday_price.parse::<f32>().unwrap();

        (today_price, yesterday_price)
    }
}

// TODO this should return box dyn error.
// That way all unwraps can be changed for ?
pub fn run_app() {
    // let (cur1, cur2) = user_input_processing::get_valid_currency_codes();
    // query_currency_api::get_exchange_rate(cur1, cur2);

    // Get yesterday's date
    let yesterday = chrono::Utc::now() - chrono::Duration::days(1);
    let yesterday_formatted = yesterday.format("%Y-%m-%e").to_string();

    // TODO don't unwrap
    let exchange_rates_raw = price_operations::get_exchange_rate_raw(
        "USD".to_string(),
        "CHF".to_string(),
        &yesterday_formatted,
    )
    .unwrap();

    let (today_price, yesterday_price) =
        price_operations::get_prices_from_api_response(exchange_rates_raw, &yesterday_formatted);

    println!("today: {} .. yesterday: {}", today_price, yesterday_price);

    // Compare today's price against yesterday price, print colored output accordingly
    match today_price.partial_cmp(&yesterday_price) {
        Some(Ordering::Less) => println!("{}", today_price.to_string().red()),
        Some(Ordering::Greater) => println!("{}", today_price.to_string().green()),
        Some(Ordering::Equal) => println!("{}", today_price),
        // TODO error out
        None => println!("error"),
    }
}
