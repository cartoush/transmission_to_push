extern crate transmission_rpc;

use log::{debug, error, info};
use ntfy::*;
use std::collections::HashMap;
use std::time::Duration;
use std::{env, thread};
use transmission_rpc::types::{BasicAuth, Result, Torrent, TorrentGetField};
use transmission_rpc::TransClient;

async fn notif_torrent_finished(dispatcher: &Dispatcher, torrent_name: &String) {
    let notif = Payload::new("transmission")
        .message(format!("{} download complete", torrent_name))
        .title("Transmission");
    dispatcher.send(&notif).await.unwrap();
}

async fn notif_torrent_added(dispatcher: &Dispatcher, torrent_name: &String) {
    let notif = Payload::new("transmission")
        .message(format!("{} download started", torrent_name))
        .title("Transmission");
    dispatcher.send(&notif).await.unwrap();
}

async fn torrents_get(client: &mut TransClient) -> Result<Vec<Torrent>> {
    let torrents = client
        .torrent_get(
            Some(vec![TorrentGetField::Name, TorrentGetField::IsFinished]),
            None,
        )
        .await?;

    match torrents.is_ok() {
        true => Ok(torrents.arguments.torrents),
        false => Err("failed getting torrents".into()),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut client;
    let url = env::var("TURL")?;
    if let (Ok(user), Ok(password)) = (env::var("TUSER"), env::var("TPWD")) {
        client = TransClient::with_auth(url.parse()?, BasicAuth { user, password });
    } else {
        client = TransClient::new(url.parse()?);
    }
    let mut duration;

    let dispatcher = Dispatcher::builder(env::var("NURL")?)
        .credentials(Auth::new(env::var("NUSER")?, env::var("NPWD")?)) // Add optional credentials
        .build()?; // Build dispatcher

    let mut old_torrents: HashMap<String, bool> = HashMap::new();
    loop {
        let torrents = match torrents_get(&mut client).await {
            Ok(torr) => {
                duration = Duration::from_secs(60);
                torr
            }
            Err(e) => {
                duration = Duration::from_secs(60 * 5);
                error!("Error : {:?}, setting interval to 300 seconds", e);
                vec![]
            }
        };

        let status: Vec<(String, bool)> = torrents
            .iter()
            .map(|it| (it.name.clone().unwrap(), it.is_finished.unwrap()))
            .collect();
        debug!("{:#?}", status);

        for i in &status {
            if old_torrents.contains_key(&i.0) {
                if i.1 && !old_torrents[&i.0] {
                    notif_torrent_finished(&dispatcher, &i.0).await;
                    info!("Torrent finished: {}", &i.0);
                }
            } else {
                notif_torrent_added(&dispatcher, &i.0).await;
            }
        }

        let _ = status
            .into_iter()
            .map(|torr| old_torrents.insert(torr.0, torr.1));
        thread::sleep(duration);
    }
}
