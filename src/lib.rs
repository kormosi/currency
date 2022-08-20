use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process;

use colored::Colorize;
use rusqlite::Connection;
use user_input_processing::is_input_valid_currency_pair;

use crate::sql_operations::insert_record_into_db;

pub struct Record {
    pub currency1: String,
    pub currency2: String,
    pub rate: f32,
    pub color: String,
}

fn construct_currency_vector() -> Vec<String> {
    let file = File::open("codes_unique_sorted.txt").expect("Error reading file");
    let reader = BufReader::new(file);

    // Some turbofish magic, I don't know
    reader.lines().collect::<Result<_, _>>().unwrap()
}

mod user_input_processing {
    use regex::Regex;
    use std::io;

    pub fn get_user_input() -> String {
        println!("Enter currency pair. 'h' for history, 'q' to quit:");
        let mut input = String::new();

        io::stdin()
            .read_line(&mut input)
            .expect("Couldn't read line");

        input.trim().to_string()
    }

    fn is_input_in_valid_format(user_input: &str) -> Option<(String, String)> {
        let re = Regex::new(r"^(?P<cur1>[a-zA-Z]{3}) (?P<cur2>[a-zA-Z]{3})$")
            .expect("Couldn't construct regex");

        let captures = re.captures(&user_input)?;
        Some((captures["cur1"].to_string(), captures["cur2"].to_string()))
    }

    pub fn is_input_valid_currency_pair(user_input: &str) -> Option<(String, String)> {
        // Vector of all valid currency codes
        // TODO should be constructed only once
        let currency_vector = crate::construct_currency_vector();

        if let Some(currency_pair) = is_input_in_valid_format(user_input) {
            if !currency_vector.contains(&currency_pair.0)
                || !currency_vector.contains(&currency_pair.1)
            {
                println!("Invalid currency code");
                return None;
            }
            return Some(currency_pair);
        } else {
            println!("Invalid input format");
            None
        }
    }
}

mod price_operations {
    use crate::Record;
    use colored::Colorize;
    use serde_json::{self, Value};
    use std::cmp::Ordering;
    use std::collections::HashMap;
    use std::{env, error::Error};

    pub fn get_price_struct(
        cur1: &str,
        cur2: &str,
        yesterday_date_formatted: &str,
    ) -> Result<(Record), Box<dyn Error>> {
        let exchange_rates_raw =
            get_exchange_rate_raw(cur1, cur2, &yesterday_date_formatted).unwrap();

        let (today_price, yesterday_price) =
            get_prices_from_api_response(cur1, cur2, exchange_rates_raw, &yesterday_date_formatted);

        // Determine color of the price change
        let color: String;

        match today_price.partial_cmp(&yesterday_price) {
            Some(Ordering::Less) => color = "red".to_string(),
            Some(Ordering::Greater) => color = "green".to_string(),
            Some(Ordering::Equal) => color = "normal".to_string(),
            // TODO don't panic
            None => panic!("error determining color"),
        }

        Ok(Record {
            currency1: cur1.to_string(),
            currency2: cur2.to_string(),
            rate: today_price,
            color: color,
        })
    }

    pub fn print_exchange_rate(currency_pair: (String, String), yesterday_date: String) {
        let exchange_rates_raw =
            get_exchange_rate_raw(&currency_pair.0, &currency_pair.1, &yesterday_date).unwrap();

        let (today_price, yesterday_price) = get_prices_from_api_response(
            &currency_pair.0,
            &currency_pair.1,
            exchange_rates_raw,
            &yesterday_date,
        );

        // println!("today: {} .. yesterday: {}", today_price, yesterday_price);
        compare_and_print_exchange_rate(today_price, yesterday_price);
    }

    // Returns raw response from the API
    fn get_exchange_rate_raw(
        cur1: &str,
        cur2: &str,
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
        cur1: &str,
        cur2: &str,
        exchange_rates_raw: (String, String),
        yesterday_date: &str,
    ) -> (f32, f32) {
        // Convert the raw strings from the API response into hashmaps
        let today_price_map: HashMap<String, Value> =
            serde_json::from_str(&exchange_rates_raw.0).expect("JSON was not well-formatted");
        let yesterday_price_map: HashMap<String, Value> =
            serde_json::from_str(&exchange_rates_raw.1).expect("JSON was not well-formatted");

        // Get the numerical values of the exchange rates
        // let today_price = today_price_map.get("USD_CHF").unwrap().to_string();
        let formatted_currency_pair = format!("{}_{}", cur1, cur2);

        let today_price = today_price_map
            .get(&formatted_currency_pair)
            .unwrap()
            .to_string();

        let yesterday_price = yesterday_price_map
            .get(&formatted_currency_pair)
            .unwrap()
            .get(yesterday_date)
            .unwrap()
            .to_string();

        // Convert the string values to floats
        let today_price = today_price.parse::<f32>().unwrap();
        let yesterday_price = yesterday_price.parse::<f32>().unwrap();

        (today_price, yesterday_price)
    }

    pub fn compare_and_print_exchange_rate(today_price: f32, yesterday_price: f32) {
        // Compare today's price against yesterday price, print colored output accordingly
        match today_price.partial_cmp(&yesterday_price) {
            Some(Ordering::Less) => println!("{}", today_price.to_string().red()),
            Some(Ordering::Greater) => println!("{}", today_price.to_string().green()),
            Some(Ordering::Equal) => println!("{}", today_price),
            // TODO error out
            None => println!("error"),
        }
    }
}

mod sql_operations {
    use crate::Record;
    use colored::Colorize;
    use rusqlite::Connection;

    pub fn insert_record_into_db(record: &Record) {
        let conn = Connection::open("db.sqlite3").unwrap();
        let mut stmt = conn
            .prepare("INSERT INTO history (cur1, cur2, rate, color) VALUES (?1, ?2, ?3, ?4)")
            .unwrap();

        stmt.execute([
            &record.currency1,
            &record.currency2,
            &record.rate.to_string(),
            &record.color,
        ])
        .unwrap();
    }

    pub fn get_history_from_db() {
        let conn = Connection::open("db.sqlite3").unwrap();
        let mut stmt = conn
            .prepare("SELECT cur1, cur2, rate, color FROM history")
            .unwrap();

        let history_iter = stmt
            .query_map([], |row| {
                Ok(Record {
                    currency1: row.get(0)?,
                    currency2: row.get(1)?,
                    rate: row.get(2)?,
                    color: row.get(3)?,
                })
            })
            .unwrap();

        for rec in history_iter {
            let color_from_db = rec.as_ref().unwrap().color.as_str();
            let rate_from_db = rec.as_ref().unwrap().rate.to_string();

            match color_from_db {
                "red" => println!("{}", rate_from_db.red()),
                "green" => println!("{}", rate_from_db.green()),
                _ => println!("{}", rate_from_db),
            }
        }
    }
}

// TODO this should return box dyn error.
// That way all unwraps can be changed for ?
pub fn run_app() {
    // TODO construct currency vector before running the code
    // - now it is constructed every time user inputs a currency pair

    loop {
        let user_input = user_input_processing::get_user_input();
        if user_input == "h" {
            sql_operations::get_history_from_db()
        } else if user_input == "q" {
            process::exit(0);
            // TODO refactor to struct
        } else if let Some(currency_pair) = is_input_valid_currency_pair(&user_input.to_uppercase())
        {
            // Get yesterday's date and format it
            let yesterday_date = chrono::Utc::now() - chrono::Duration::days(1);
            let yesterday_date_formatted = yesterday_date.format("%Y-%m-%d").to_string();

            let record = price_operations::get_price_struct(
                &currency_pair.0,
                &currency_pair.1,
                &yesterday_date_formatted,
            )
            .unwrap();

            match record.color.as_str() {
                "red" => println!("{}->{}: {}", record.currency1, record.currency2, record.rate.to_string().red()),
                "green" => println!("{}->{}: {}", record.currency1, record.currency2, record.rate.to_string().green()),
                _ => println!("{}->{}: {}", record.currency1, record.currency2, record.rate)
            }

            // insert into db
            insert_record_into_db(&record);
        }
    }
}
