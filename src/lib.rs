use std::collections::HashMap;
use std::{env, process};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::io::prelude::*;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use curl::easy::Easy;
use serde_json::Value;

const INET_DL_ADDR: &str = "https://cdn.wahrungsrechner.info/api/latest.json";
const DEFAULT_FILENAME: &str = "currency.json";

#[derive(Debug)]
enum ArgumentResult {
    Success,
    SuccessAndUsualList,
    SuccessAndCompleteList,
    ArgumentError,
}

#[derive(Clone, Debug)]
struct ExchangeProcess {
    from: String,
    to: String,
    rate: f64,
    amount_from: f64,
    amount_to: f64,
}

impl ExchangeProcess {
    fn new() -> ExchangeProcess {
        ExchangeProcess {
            from: String::new(),
            to: String::new(),
            rate: 0.0,
            amount_from: 0.0,
            amount_to: 0.0,
        }
    }
}

pub fn run() -> bool {

    let mut rates: HashMap<String, f64> = HashMap::new();
    let mut exchange = ExchangeProcess::new();

    if !check_rates_file() {
        if !download_rates_file() {
            eprintln!("Error downloading the currency data.");
            process::exit(1);
        }
    }

    if !load_rates_file_from_disk(&mut rates) {
        eprintln!("Error loading currency data from disk.");
        process::exit(2);
    }

    let func = parse_arguments(&mut exchange);
    match func {
        ArgumentResult::ArgumentError => process::exit(3),
        ArgumentResult::SuccessAndUsualList => {
            print_usual_rates(&rates);
            process::exit(0)
        }
        ArgumentResult::SuccessAndCompleteList => {
            print_all_rates(&rates);
            process::exit(0)
        }
        _ => (),
    }

    if !rates.contains_key(&exchange.from) {
        println!("Did not found currency {}.", exchange.from);
        process::exit(4)
    }
    if !rates.contains_key(&exchange.to) {
        println!("Did not found currency {}.", exchange.to);
        process::exit(5)
    }

    exchange.rate = rates[&exchange.to] / rates[&exchange.from];
    exchange.amount_to = exchange.amount_from * exchange.rate;
    //dbg!(&exchange);

    println!("\x1B[24mActual exchange rate:\x1B[0m \x1B[92m{}\x1B[39m \x1B[93m{:.4}\x1B[39m = \x1B[92m{}\x1B[39m \x1B[93m{:.4}\x1B[39m",
             exchange.from,
             exchange.amount_from,
             exchange.to,
             exchange.amount_to
             );

    true

}

fn check_rates_file() -> bool {

    let file_name = Path::new(get_temp_dir().as_str()).join(DEFAULT_FILENAME);
    if !file_name.exists() {
        println!("A local copy of {} didn't exist.", file_name.display());
        return false;
    }

    let file = match File::open(&file_name) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Couldn't open {} (error: {}).", file_name.display(), err);
            return false
        },
    };

    let metadata = match file.metadata() {
        Ok(metadata) => metadata,
        Err(err) => {
            eprintln!("Couldn't get metadata from file {} (error: {}).", file_name.display(), err);
            return false
        },
    };

    let mut file_date: u64 = 0;
    if let Ok(time) = metadata.modified() {
        match time.duration_since(UNIX_EPOCH) {
            Ok(t) => file_date = t.as_secs(),
            _ => (),
        }
    }

    let mut cur_date: u64 = 0;
    let now = SystemTime::now();
    match now.duration_since(UNIX_EPOCH) {
        Ok(t) => cur_date = t.as_secs(),
        _ => (),
    }

    if cur_date - file_date >= 3_600 {
        return false;
    }

    true

}

fn download_rates_file() -> bool {

    let file_name = Path::new(get_temp_dir().as_str()).join(DEFAULT_FILENAME);
    let file = match File::create(&file_name) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Couldn't create {} (error: {}).", file_name.display(), err);
            return false;
        },
    };

    let mut writer = BufWriter::new(file);

    let mut handle = Easy::new();
    handle.url(INET_DL_ADDR).unwrap();

    let mut transfer = handle.transfer();
    transfer.write_function(|data| {
        writer.write_all(data).unwrap();
        Ok(data.len())
    }).unwrap();

    let recv = match transfer.perform() {
        Err(err) => {
            eprintln!("Error while download: {}", err);
            return false
        }
        Ok(recv) => recv,
    };
    //dbg!(&recv);
    true

}

fn load_rates_file_from_disk (exchange_rates: &mut HashMap<String, f64>) -> bool {

    let file_name = Path::new(get_temp_dir().as_str()).join(DEFAULT_FILENAME);
    let file = match File::open(&file_name) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Couldn't open {} (error: {}).", file_name.display(), err);
            return false
        },
    };

    let mut content = String::new();
    let reader = BufReader::new(file);
    for line in reader.lines() {

        let l = line.unwrap_or_default();
        content.push_str(&l);

    }

    if content.len() == 0 {
        eprintln!("File is empty.");
        return false;
    }

    let json: Value = serde_json::from_str(&content).unwrap();
    let rates = json.as_object()
        .and_then(|object| object.get("rates"))
        .and_then(|rates| rates.as_object())
        .unwrap();

    for rate in rates.iter() {
        let key: String = rate.0.to_string();
        let val: f64 = rate.1.as_f64().unwrap();
        exchange_rates.insert(key, val);
    }

    true
}

pub fn get_currency_name(currency: &str) -> String {
    let result: String;
    match currency {
        "EUR" => result = "Euro".to_string(),
        "USD" => result = "US Dollar".to_string(),
        "JPY" => result = "Japanese Yen".to_string(),
        "BGN" => result = "Bulgarian Lev".to_string(),
        "CZK" => result = "Czech Koruna".to_string(),
        "DKK" => result = "Danish Krone".to_string(),
        "GBP" => result = "Pound Sterling".to_string(),
        "HUF" => result = "Hungarian Forint".to_string(),
        "PLN" => result = "Polish Zloty".to_string(),
        "RON" => result = "Romanian Leu".to_string(),
        "SEK" => result = "Swedish Krona".to_string(),
        "CHF" => result = "Swiss Franc".to_string(),
        "ISK" => result = "Islandic Krona".to_string(),
        "NOK" => result = "Norwegian Krone".to_string(),
        "TRY" => result = "Turkish Lira".to_string(),
        "AUD" => result = "Australian Dollar".to_string(),
        "BRL" => result = "Brazilian Real".to_string(),
        "CAD" => result = "Canadian Dollar".to_string(),
        "CNY" => result = "Chinese Yuan Renmimbi".to_string(),
        "HKD" => result = "Hong Kong Dollar".to_string(),
        "IDR" => result = "Indonesian Rupiah".to_string(),
        "ILS" => result = "Israeli Shekel".to_string(),
        "INR" => result = "Indian Rupee".to_string(),
        "KRW" => result = "South Korean Won".to_string(),
        "MXN" => result = "Mexican Peso".to_string(),
        "MYR" => result = "Malaysian Ringgit".to_string(),
        "NZD" => result = "New Zealand Dollar".to_string(),
        "PHP" => result = "Philippine Peso".to_string(),
        "SGD" => result = "Singapore Dollar".to_string(),
        "THB" => result = "Thai Baht".to_string(),
        "ZAR" => result = "South African Rand".to_string(),
        _ => result = String::from("Unknown"),
    }
    result
}

fn get_temp_dir() -> String {
    #[cfg(target_os="windows")]
    let d = env::var("TEMP").unwrap_or_else(|err| {
        eprintln!("could not find %TEMP%: {}", err);
        String::from(".")
    });
    #[cfg(target_os="linux")]
    let d = String::from("/tmp");
    d
}

fn parse_arguments(exchange: &mut ExchangeProcess) -> ArgumentResult {

    let prg_name = env::args().nth(0).unwrap();
    let version = option_env!("CARGO_PKG_VERSION").unwrap();

    let mut params = env::args().skip(1);

    if params.len() == 0 {
        println!("{} needs three arguments or try --help.", prg_name);
        std::process::exit(1);
    }

    let mut pos: usize = 0;
    while let Some(param) = params.next() {

        match &param[..] {

            "-h" | "--help" => {
                print_help(&prg_name);
                std::process::exit(0);
            }

            "-V" | "--version" => {
                println!("{} v{}\n", prg_name, version);
                std::process::exit(0);
            }

            "-lu" | "--list-usual" | "-l" | "--list" => {
                return ArgumentResult::SuccessAndUsualList;
            }

            "-la" | "--list-all" => {
                return ArgumentResult::SuccessAndCompleteList;
            }

            _ => {

                if param.starts_with('-') {
                    eprintln!("Unkown argument: {}", param);
                    return ArgumentResult::ArgumentError;
                }

                if pos == 0 {

                    exchange.from = param.to_ascii_uppercase().to_string();
                    pos += 1;

                } else if pos == 1 {

                    exchange.to = param.to_ascii_uppercase().to_string();
                    pos += 1;

                } else if pos == 2 {

                    exchange.amount_from = param.parse::<f64>().unwrap_or_default();
                    pos += 1;

                } else {
                    eprintln!("Too many arguments, try: {} --help", prg_name);
                    return ArgumentResult::ArgumentError;
                }

            }

        }

    }

    if pos == 2 {
        exchange.amount_from = 1.0;
        pos += 1;
    }

    if pos != 3 {
        eprintln!("Not enough arguments, try: {} --help", prg_name);
        std::process::exit(1);
    }

    ArgumentResult::Success

}

fn print_usual_rates(rates: &HashMap<String, f64>) {

    let mut sorted: Vec<_> = rates.iter().collect();
    sorted.sort_by_key(|a| a.0);

    println!("\x1B[1mUsual exchange rates:\n---------------------\x1B[0m\n");

    println!(" Abbr| Currency Name\n-----|----------------------");
    for (key, _) in sorted.iter() {
        let rate_name = get_currency_name(&key);
        if rate_name != "Unknown" {
            println!(" {} | {}", key, rate_name);
        }
    }

    println!("\n\x1B[1mUse the abbreviation to calc the exchange rates.\x1B[0m")
}

fn print_all_rates(rates: &HashMap<String, f64>) {

    let mut sorted: Vec<_> = rates.iter().collect();
    sorted.sort_by_key(|a| a.0);

    println!("\x1B[1mAll available exchange rates:\n-----------------------------\x1B[0m\n");

    for (key, _) in sorted.iter() {
        print!("| {} ", key);
    }
    println!("|");

    println!("\n\x1B[1mUse the abbreviation to calc the exchange rates.\x1B[0m")

}

fn print_help(name: &str) {
    println!("\nUsage:");
    println!("{} [<OPTIONS>] [CURRENCY_FROM] [CURRENCY_TO] [AMOUNT]\n", name);
    println!("Options:");
    println!("-l,  --list        same as '--list-usual'");
    println!("-la, --list-all    list all available currencies (long list)");
    println!("-lu, --list-usual  list the usual currencies for exchange");
    println!("-h,  --help        show this help");
    println!("-V,  --version     show the program version and exit");
    println!("");
    println!("Exchange arguments:");
    println!("CURRENCY_FROM      The currency you have.");
    println!("CURRENCY_TO        The currency you want to change into.");
    println!("AMOUNT             The amount you want to change.");
    println!("");
}
