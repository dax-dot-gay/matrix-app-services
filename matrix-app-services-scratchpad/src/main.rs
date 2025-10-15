use std::time::Duration;

use matrix_app_services::{ Appservice, Config, Namespace };

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let config = Config::builder("matrix-scratchpad")
        .homeserver("dax.gay")
        .proxy_ports(9000..10000)
        .sender_localpart("matrix-scratchpad")
        .url("http://192.168.1.30:21528")
        .homeserver_token(std::env::var("SCRATCHPAD_HS_TOKEN").unwrap())
        .appservice_token(std::env::var("SCRATCHPAD_AS_TOKEN").unwrap())
        .namespace(Namespace::user("@.*"))
        .namespace(Namespace::alias("#.*"))
        .namespace(Namespace::room("#.*"))
        .local_address(([0,0,0,0], 21528))
        .build();
    let service = Appservice::new(config)?;
    std::fs::write("registration.yaml", service.config().registration_yaml().unwrap()).unwrap();
    service.serve();
    tokio::time::sleep(Duration::from_secs(2)).await;
    let client = service.build_service_client().build().await?;

    let result = client.whoami().await;
    println!("{result:?}");

    println!("Done!");
    loop {}
}
