use std::collections::HashMap;

use reqwest::blocking as request;
use serde::Deserialize;

use crate::{logger, parser, render, system};

#[derive(Deserialize)]
struct Response {
    records: Vec<Record>,
}

#[derive(Deserialize)]
struct Record {
    id: u32,
    size: f64,
    movie: Option<NestedRecord>,
    series: Option<NestedRecord>,
    timeleft: Option<String>,
}

#[derive(Deserialize)]
struct NestedRecord {
    title: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Torrent {
    pub id: u32,
    pub name: String,
    pub size: u64,
    pub eta: u64,
}

// Obtains Torrents from Radarr or Sonarr.
pub fn get(url: &String, platform: &String) -> Vec<Torrent> {
    // Request active torrents in queue from the Radarr or Sonarr API.
    let res: Response = match request::get(url) {
        // API can be reached.
        Ok(res) => match res.json() {
            // Response is valid.
            Ok(res) => res,
            // Did not respond with valid JSON.
            Err(error) => {
                logger::alert(
                    "WARN",
                    "Unable to process queue, will attempt again next run.".to_string(),
                    "The API has responded with an invalid response.".to_string(),
                    Some(error.to_string()),
                );
                // Something went wrong, return an empty queue as fallback.
                Response { records: vec![] }
            }
        },
        Err(error) => {
            logger::alert(
                "WARN",
                "Unable to process queue, will attempt again next run.".to_string(),
                "The connection to the API was unsuccessful.".to_string(),
                Some(error.to_string()),
            );
            // Something went wrong, return an empty queue as fallback.
            Response { records: vec![] }
        }
    };

    let mut torrents: Vec<Torrent> = vec![];

    // Iterate over all torrents.
    res.records.iter().for_each(|record| {
        // Obtain HMS from timeleft attribute.
        let timeleft = record.timeleft.clone().unwrap_or_else(|| "0".to_string());

        // Convert timeleft from HMS to milliseconds.
        let timeleft_ms = parser::string_hms_to_ms(&timeleft);

        // Extract name from API record, if it fails return "Unknown".
        let name: String = match platform.as_str() {
            "radarr" => record
                .movie
                .as_ref()
                .map(|nested| nested.title.clone())
                .unwrap_or_else(|| String::from("Unknown")),
            "sonarr" => record
                .series
                .as_ref()
                .map(|nested| nested.title.clone())
                .unwrap_or_else(|| String::from("Unknown")),
            _ => String::from("Unknown"),
        };

        // Add torrent to the list.
        torrents.push(Torrent {
            id: record.id,
            name,
            size: record.size as u64,
            eta: timeleft_ms,
        });
    });

    torrents
}

// Determines if the torrent is eligible to be striked.
pub fn process(queue_items: Vec<Torrent>, strikelist: &mut HashMap<u32, u32>, env: &system::Envs) {
    // Table rows that will be pretty-printed to the terminal.
    let mut table_contents: Vec<render::TableContent> = vec![];

    // Loop over all active torrents from the queue.
    for torrent in queue_items {
        let id = torrent.id.clone();
        let mut status = String::from("Normal");

        // Add torrent id to strikes with default "0" if it does not exist yet.
        let mut strikes: u32 = match strikelist.get(&id) {
            Some(strikes) => strikes.clone(),
            None => {
                strikelist.insert(id, 0);
                0
            }
        };

        // -- Bypass Rules -- Rules that define if a torrent is eligible to be striked.

        let mut bypass: bool = false;

        // Torrent is being processed or the time is infinite.
        if torrent.eta == 0 && !env.aggresive_strikes {
            status = String::from("Pending");
            bypass = true;
        }

        // Torrent is larger than set threshold.
        let size_threshold_bytes = parser::string_bytesize_to_bytes(&env.size_threshold);
        if torrent.size >= size_threshold_bytes {
            status = String::from("Ignored");
            bypass = true;
        }

        // -- Strike rules -- Rules that define when to strike a torrent.

        if !bypass {
            // Torrent will take longer than set threshold.
            let time_threshold_ms = parser::string_hms_to_ms(&env.time_threshold);
            if (torrent.eta >= time_threshold_ms) || (torrent.eta == 0 && env.aggresive_strikes) {
                // Increment strikes if it's below set maximum.
                if strikes < env.strike_threshold {
                    strikes += 1;
                    strikelist.insert(id, strikes);
                }
                status = String::from("Striked");
            }

            // Torrent meets set amount of strikes, a request to delete will be sent.
            if strikes >= env.strike_threshold {
                delete(&format!(
                    "{}/api/v3/queue/{}?blocklist=true&apikey={}",
                    env.baseurl, id, env.apikey
                ));
                status = String::from("Removed");
            }
        }

        // -- Logging --

        // Add torrent to pretty-print table.
        table_contents.push(render::TableContent {
            strikes: format!("{}/{}", strikes, env.strike_threshold),
            status,
            name: torrent.name.chars().take(32).collect::<String>(),
            eta: parser::ms_to_eta_string(&torrent.eta),
            size: format!("{:.2} GB", (torrent.size as f64 / 1000000000.0)).to_string(),
        })
    }

    // Print table to terminal.
    render::table(&table_contents);
}

// -- Deletes Torrent from Radarr or Sonarr.
pub fn delete(url: &String) {
    // Send the request to delete to the API.
    match request::Client::new().delete(url).send() {
        // Should be deleted.
        Ok(_) => (),
        // Attempt to delete did not go through. (This should be attempted again next run)
        Err(error) => {
            logger::alert(
                "WARN",
                "Failed to remove torrent, will attempt again next run.".to_string(),
                "The API has refused this request.".to_string(),
                Some(error.to_string()),
            );
        }
    }
}
