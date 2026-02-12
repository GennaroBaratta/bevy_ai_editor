use reqwest::{Client, Proxy};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let proxy_url = "http://127.0.0.1:8045";
    let target_url = "https://www.google.com";

    println!("Starting Proxy Check Example");
    println!("----------------------------");
    
    // Configure Proxy
    println!("Configuring proxy: {}", proxy_url);
    let proxy = Proxy::all(proxy_url)?;
    
    // Build Client
    let client = Client::builder()
        .proxy(proxy)
        .build()?;

    // Send Request
    println!("Sending GET request to: {}", target_url);
    
    // We'll wrap the request in a match to handle connection errors explicitly
    match client.get(target_url).send().await {
        Ok(response) => {
            println!("Response Status: {}", response.status());
            if response.status().is_success() {
                println!("Proxy Check Passed: Connectivity verified.");
                Ok(())
            } else {
                println!("Proxy Check Failed: Non-success status code.");
                Err(format!("Request failed with status: {}", response.status()).into())
            }
        },
        Err(e) => {
            println!("Proxy Check Failed: Connection error.");
            println!("Error details: {}", e);
            Err(e.into())
        }
    }
}
