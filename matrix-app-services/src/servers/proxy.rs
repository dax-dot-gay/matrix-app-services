use crate::{ client::Appservice, Config };

pub async fn serve_proxy(
    service: Appservice,
    proxy_port: u16,
    cert: String,
    key: String
) -> crate::Result<()> {
    loop {
    }
}
