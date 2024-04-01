extern crate transmission_rpc;

use ntfy::*;
use std::env;
use transmission_rpc::types::{BasicAuth, Result, TorrentGetField};
use transmission_rpc::TransClient;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client;
    let url = env::var("TURL")?;
    if let (Ok(user), Ok(password)) = (env::var("TUSER"), env::var("TPWD")) {
        client = TransClient::with_auth(url.parse()?, BasicAuth { user, password });
    } else {
        client = TransClient::new(url.parse()?);
    }

    let res = client
        .torrent_get(
            Some(vec![TorrentGetField::Name, TorrentGetField::IsFinished]),
            None,
        )
        .await?;
    let status: Vec<(String, bool)> = res
        .arguments
        .torrents
        .iter()
        .map(|it| (it.name.clone().unwrap(), it.is_finished.unwrap()))
        .collect();
    println!("{:#?}", status);

    let dispatcher = Dispatcher::builder(env::var("NURL")?)
        .credentials(Auth::new(env::var("NUSER")?, env::var("NPWD")?)) // Add optional credentials
        .build()?; // Build dispatcher

    let payload = Payload::new("mytopic")
        .message("Hello, **World**!") // Add optional message
        .title("Alert") // Add optional title
        .priority(Priority::High) // Edit priority
        .click(Url::parse("https://transmission.pennarbed.eu")?) // Add optional clickable url
        .markdown(true); // Use markdown

    dispatcher.send(&payload).await.unwrap();

    Ok(())
}
