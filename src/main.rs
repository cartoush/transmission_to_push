extern crate transmission_rpc;

use log::{debug, error, info};
use ntfy::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::time::Duration;
use std::{env, thread};
use transmission_rpc::types::{BasicAuth, Result, Torrent, TorrentGetField};
use transmission_rpc::TransClient;

#[derive(Serialize, Deserialize, Debug)]
struct TorrentStatus {
    name: String,
    is_finished: bool,
}

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
    env_logger::init();

    let mut client;
    let url = env::var("TURL")?;
    if let (Ok(user), Ok(password)) = (env::var("TUSER"), env::var("TPWD")) {
        client = TransClient::with_auth(url.parse()?, BasicAuth { user, password });
    } else {
        client = TransClient::new(url.parse()?);
    }
    let mut duration = Duration::from_secs(0);

    let dispatcher = Dispatcher::builder(env::var("NURL")?)
        .credentials(Auth::new(env::var("NUSER")?, env::var("NPWD")?)) // Add optional credentials
        .build()?; // Build dispatcher

    let mut old_torrents: HashMap<String, bool> = HashMap::new();
    if Path::new("cache.cbor").exists() {
        let cache_file = File::open("cache.cbor")?;
        let reader = BufReader::new(cache_file);
        let cached: Vec<TorrentStatus> = serde_cbor::from_reader(reader).unwrap();

        for i in cached {
            old_torrents.insert(i.name, i.is_finished);
        }
    }

    loop {
        thread::sleep(duration);
        debug!("Old torrents: {:?}", old_torrents);

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

        let status: Vec<TorrentStatus> = torrents
            .iter()
            .map(|it| TorrentStatus {
                name: it.name.clone().unwrap(),
                is_finished: it.is_finished.unwrap(),
            })
            .collect();

        for i in &status {
            debug!("{} {}", i.name, i.is_finished);

            if old_torrents.contains_key(&i.name) {
                if i.is_finished && !old_torrents[&i.name] {
                    notif_torrent_finished(&dispatcher, &i.name).await;
                    info!("Torrent finished: {}", &i.name);
                }
            } else if !i.is_finished {
                notif_torrent_added(&dispatcher, &i.name).await;
                info!("Torrent added: {}", &i.name);
            }
        }

        {
            let cache_file = File::create("cache.cbor")?;
            let writer = BufWriter::new(cache_file);
            serde_cbor::to_writer(writer, &status).unwrap();
        }
        old_torrents = status
            .into_iter()
            .map(|torr| (torr.name, torr.is_finished))
            .collect::<HashMap<String, bool>>();
    }
}
