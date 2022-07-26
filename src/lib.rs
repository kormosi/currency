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

        input.trim().to_uppercase().to_string()
    }

    fn is_input_in_valid_format(user_input: String) -> Option<(String, String)> {
        let re = Regex::new(r"^(?P<cur1>[a-zA-Z]{3}) (?P<cur2>[a-zA-Z]{3})$")
            .expect("Couldn't construct regex");

        let captures = re.captures(&user_input)?;
        Some((captures["cur1"].to_string(), captures["cur2"].to_string()))
    }

    /// A wrapper function around the above get_user_input() and
    /// dissect_user_input() functions. Guaranteed to return a valid
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

mod query_currency_api {
    use chrono::{Duration, Utc};
    use std::env;

    pub fn get_exchange_rate(cur1: String, cur2: String) {
        let api_key =
            env::var("CURRENCY_API_KEY").expect("CURRENCY_API_KEY environment variable not set");

        let yesterday = Utc::now() - Duration::days(1);

        let url_today = format!(
            "https://free.currconv.com/api/v7/convert?q={}_{}&compact=ultra&apiKey={}",
            cur1, cur2, api_key
        );

        let url_yesterday = format!(
            "https://free.currconv.com/api/v7/convert?q={}_{}&compact=ultra&date={}&apiKey={}",
            cur1,
            cur2,
            yesterday.format("%Y-%m-%e"),
            api_key,
        );

        // TODO change unwrap to at least expect
        let resp_today = reqwest::blocking::get(url_today).unwrap().text().unwrap();
        println!("{:#?}", resp_today);

        // TODO change unwrap to at least expect
        let resp_yesterday = reqwest::blocking::get(url_yesterday)
            .unwrap()
            .text()
            .unwrap();
        println!("{:#?}", resp_yesterday);
    }
}

pub fn run_app() {
    let (cur1, cur2) = user_input_processing::get_valid_currency_codes();
    query_currency_api::get_exchange_rate(cur1, cur2);
}
