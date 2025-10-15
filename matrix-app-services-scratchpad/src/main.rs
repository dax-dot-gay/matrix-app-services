use std::time::Duration;

use matrix_app_services::{ Appservice, Config };

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::builder("matrix-scratchpad")
        .homeserver("dax.gay")
        .proxy_ports(9000..10000)
        .sender_localpart("matrix-scratchpad")
        .url("http://localhost:8080")
        .build();
    let service = Appservice::new(config)?;
    service.serve();
    tokio::time::sleep(Duration::from_secs(2)).await;
    let client = service.build_service_client().build().await?;

    let result = client.whoami().await;
    println!("{result:?}");

    println!("Done!");

    Ok(())
}
