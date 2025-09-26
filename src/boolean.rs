use std::collections::HashMap;
use std::hash::Hash;
use tokio;

use rspotify::{model::*, prelude::*, ClientCredsSpotify, Credentials};

mod track;

fn playlist_id(id: &str, num: u8) -> PlaylistId<'_> {
    PlaylistId::from_id(id).expect(&format!("Invalid ID for playlist {}: {}", num, id))
}

#[tokio::main]
async fn main() {
    // May require the `env-file` feature enabled if the environment variables
    // aren't configured manually.
    // let creds = Credentials::from_env().unwrap();
    let creds = Credentials::new(
        "8d52127f8d8c4edeb2b44917c22bafc2",
        "e42314f259d942899b5a78d45e4c9ee3",
    );
    // let oauth = OAuth::from_env(scopes!("user-library-read")).unwrap();

    let spotify = ClientCredsSpotify::new(creds);

    // This function requires the `cli` feature enabled.
    spotify.request_token().await.unwrap();

    let playlist_items = async |id: PlaylistId| {
        spotify
            .playlist_items_manual(id, None, None, None, None)
            .await
    };

    // Typical iteration, no extra boilerplate needed.
    let playlist_1_id = playlist_id("5OH6M4lM3i7EcZZyxAiwN0", 1);
    let playlist_2_id = playlist_id("6Hm1s5RVZtuZmqFm82YvcT", 2);
    let playlist_3_id = playlist_id("1Fx1khIihbsq47XVcZiIzq", 3);
    let result_1 = playlist_items(playlist_1_id).await;
    let result_2 = playlist_items(playlist_2_id).await;
    let result_3 = playlist_items(playlist_3_id).await;
    println!("Items:");
    if let (Ok(page_1), Ok(page_2), Ok(page_3)) = (result_1, result_2, result_3) {
        let tracks_1 = tabulate_tracks(track::cat_tracks(page_1.items));
        let tracks_2 = tabulate_tracks(track::cat_tracks(page_2.items));
        let tracks_3 = tabulate_tracks(track::cat_tracks(page_3.items));
        let intersection = intersect_tables(tracks_1, tracks_2);
        let difference = table_difference(intersection, tracks_3);
        for track in difference.values() {
            print!(
                "{}",
                match track.artists.first() {
                    Some(artist) => &artist.name,
                    None => "Unknown artist",
                }
            );
            if let Some(tail) = track.artists.get(1..) {
                for artist in tail {
                    print!(", {}", artist.name);
                }
            }
            println!(" - \"{}\"", track.name);
        }
    }
}

// fn tabulate_tracks(tracks: Vec<(usize, FullTrack)>) -> HashMap<TrackId<'static>, FullTrack> {
fn tabulate_tracks(tracks: Vec<FullTrack>) -> HashMap<TrackId<'static>, FullTrack> {
    let mut table = HashMap::new();
    // for (_, t) in &tracks {
    for t in &tracks {
        if let Some(id) = &t.id {
            table.insert(id.clone_static(), t.clone());
        }
    }
    return table;
}

fn intersect_tables<K, V>(table_1: HashMap<K, V>, table_2: HashMap<K, V>) -> HashMap<K, V>
where
    K: Clone,
    K: Eq,
    K: Hash,
    V: Clone,
{
    let mut intersection: HashMap<K, V> = HashMap::new();
    for (k, v) in &table_1 {
        if table_1.contains_key(&k) && table_2.contains_key(&k) {
            intersection.insert(k.clone(), v.clone());
        }
    }
    return intersection;
}

fn table_difference<K, V>(table_1: HashMap<K, V>, table_2: HashMap<K, V>) -> HashMap<K, V>
where
    K: Clone,
    K: Eq,
    K: Hash,
    V: Clone,
{
    let mut difference: HashMap<K, V> = HashMap::new();
    for (k, v) in &table_1 {
        if table_1.contains_key(&k) && !table_2.contains_key(&k) {
            difference.insert(k.clone(), v.clone());
        }
    }
    return difference;
}
