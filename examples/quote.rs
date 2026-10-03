use insightsentry::Client;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let symbol = std::env::args()
        .nth(1)
        .ok_or("Usage: quote <provider-symbol>; bearer token on stdin")?;
    let token = std::io::read_to_string(std::io::stdin())?;
    let response = Client::new(token.trim()).quotes(&[&symbol]).await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
