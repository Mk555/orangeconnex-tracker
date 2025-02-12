use reqwest::{Client, Error};
use serde_json::{json, Value};
use std::{env, thread, time};

const API_ADDRESS: &str = "https://azure-cn.orangeconnex.com/oc/capricorn-website/website/v1/tracking/traces";

/*
    FUNCTIONS
*/

fn parse_json(json_string: String) -> Result<Value, serde_json::Error> {
    let parsed: Value = serde_json::from_str(&json_string)?;
    Ok(parsed)
}

async fn send_telegram_message(bot_token: &str, chat_id: &str, message: &str) -> Result<(), reqwest::Error> {
    let api_url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

    let params = json!({
        "chat_id": chat_id,
        "text": message
    });

    let client = Client::new();
    let response = client.post(&api_url)
        .json(&params)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Message sent successfully!");
    } else {
        println!("Failed to send message: {:?}", response.text().await?);
    }

    Ok(())
}

/*
    MAIN
*/
#[tokio::main]
async fn main() -> Result<(), Error>  {
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").expect("Missing env variable TELEGRAM_BOT_TOKEN");
    let chat_id = env::var("TELEGRAM_CHAT_ID").expect("Missing env variable TELEGRAM_CHAT_ID");
    let order_id = env::var("ORDER_ID").expect("Missing env variable ORDER_ID");
    let sleep_time_str = env::var("SLEEP_TIME").expect("Missing env variable SLEEP_TIME");
    let sleep_time = sleep_time_str.parse().expect("Failed to parse SLEEP_TIME in integer");

    println!("-- Start --");
    let http_client = Client::new();

    let mut last_status: String = String::from("");

    let params = json!({
        "trackingNumbers": order_id,
        "language": "en-US",
    });

    loop {

        let response = http_client.post(API_ADDRESS)
            .header("Content-Type", "application/json")
            .json(&params)
            .send().await.unwrap();

        if response.status() != 200 {
            println!("HTTP Error code : {:?}", response.status());
            continue
        }

        let package_info: Value;
        match parse_json(response.text().await.unwrap()) {
            Ok(parsed) => {
                package_info = parsed["result"]["waybills"].get(0).unwrap().clone();
            }
            Err(e) => {
                println!("Error parsing json: {:?}", e);
                continue
            }
        }
        if package_info["lastStatus"] != last_status {
            last_status = String::from(package_info["lastStatus"].as_str().unwrap());
            let mut message = format!("Status : {}\nHistory : \n", &package_info["lastStatus"].as_str().unwrap());
            let traces = package_info["traces"].as_array().unwrap().clone();
            for trace in traces {
                let mut trace_formated: String = String::from("");
                match &trace {
                    Value::Object(map) => {
                        if map.contains_key("oprCountry") && map.contains_key("oprCity") {
                            trace_formated = format!("{} : {} ({}, {})", 
                                &trace["eventDesc"].as_str().unwrap(),&trace["oprTime"].as_str().unwrap(),
                                &trace["oprCity"].as_str().unwrap(),&trace["oprCountry"].as_str().unwrap());
                        } else if map.contains_key("oprCountry") && !  map.contains_key("oprCity") {
                            trace_formated = format!("{} : {} ({})", 
                                &trace["eventDesc"].as_str().unwrap(),&trace["oprTime"].as_str().unwrap(),
                                &trace["oprCountry"].as_str().unwrap());

                        } else {
                            trace_formated = format!("{} : {}", &trace["eventDesc"].as_str().unwrap(),&trace["oprTime"].as_str().unwrap());
                        }
                    }
                    _ => println!("The value is not an object."),
                }
                message = format!("{} \t - {}\n", message, trace_formated);
            }
            println!("{}", message);
            send_telegram_message(&bot_token, &chat_id, &message).await.unwrap();
        }
        println!("-- Sleep for {:?} seconds --", sleep_time);
        thread::sleep(time::Duration::from_secs(sleep_time));
    }
    
}
