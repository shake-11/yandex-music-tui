//! obtain a token through the oauth device flow.
//!
//! ```bash
//! cargo run --example device_auth
//! ```

#[tokio::main]
async fn main() -> Result<(), yamuse::Error> {
    let client = yamuse::Client::anonymous()?;
    let options = yamuse::DeviceAuthOptions::default();

    let token = client
        .device_auth(options, |code| {
            println!(
                "1. open {}",
                code.verification_url.as_deref().unwrap_or("?")
            );
            println!("2. enter {}", code.user_code.as_deref().unwrap_or("?"));
            println!("\nwaiting for confirmation...");
        })
        .await?;

    println!(
        "\naccess token:  {}",
        token.access_token.as_deref().unwrap_or("?")
    );
    println!(
        "refresh token: {}",
        token.refresh_token.as_deref().unwrap_or("?")
    );
    println!("expires in:    {}s", token.expires_in.unwrap_or(0));
    println!("\nkeep the refresh token — the access token expires.");
    Ok(())
}
