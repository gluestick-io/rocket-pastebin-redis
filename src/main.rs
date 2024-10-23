#[macro_use] extern crate rocket;
mod paste_id;

use std::sync::Mutex;
use rocket::data::{Data, ToByteUnit};
use rocket::http::uri::Absolute;
use redis::Commands;
use serde::Deserialize;
use std::sync::LazyLock; // Add this import

// In a real application, these would be retrieved dynamically from a config.
const ID_LENGTH: usize = 12;

use paste_id::PasteId;

fn default_host() -> String {
    "http://localhost:8000".to_string()
}

fn default_redis_url() -> String {
    "redis://127.0.0.1/".to_string()
}

#[derive(Deserialize, Debug)]
struct Config {
  #[serde(default="default_host")]
  host: String,
  #[serde(default="default_redis_url")]
  redis_url: String,
}

const HOST: Absolute<'static> = uri!("http://localhost:8000");

fn save_to_valkey(id: String, value: String) -> redis::RedisResult<()> {
    // connect to redis
    let client = redis::Client::open((*GLOBAL_CONFIG.lock().unwrap()).redis_url.clone())?; // Use the constant here
    let mut con = client.get_connection()?;
    con.set(id, value) // Return the result of the set operation
}

#[post("/", data = "<paste>")]
async fn upload(paste: Data<'_>) -> Result<String, std::io::Error> { // Changed return type to Result
    let id = PasteId::new(ID_LENGTH); // Create a new instance instead of cloning
    let paste_string = paste.open(128.kibibytes()).into_string().await?.into_inner(); // Convert Capped<String> to String
    _ = save_to_valkey(id.to_string(), paste_string); // Use id directly
    Ok(uri!(HOST, retrieve(id)).to_string()) // Pass id directly
}

fn fetch_from_valkey(id: String) -> redis::RedisResult<String> {
    // connect to redis
    let client = redis::Client::open((*GLOBAL_CONFIG.lock().unwrap()).redis_url.clone())?; // Use the constant here
    let mut con = client.get_connection()?;
    con.get(id)
}

#[get("/<id>")]
async fn retrieve(id: PasteId<'_>) -> Result<String, std::io::Error> { // Changed return type to Result
    match fetch_from_valkey(id.to_string()) {
        Ok(retval) => Ok(retval),
        Err(e) => {
            println!("{}", e);
            Ok("Value not found".to_string())
        }, // Handle error
    }
}

fn load_config() -> Config {
    match envy::from_env::<Config>() {
        Ok(config) => config,
        Err(error) => panic!("{:#?}", error)
    }    
}

static GLOBAL_CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| Mutex::new(load_config())); // Use LazyLock

#[launch]
fn rocket() -> _ {

    rocket::build().mount("/", routes![retrieve, upload])
}
