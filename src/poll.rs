use rspotify::{model::*, prelude::*, model::PlayableItem::Track, ClientCredsSpotify, Credentials};
use differ::{Differ, Span, Tag::*};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials as LettreCredentials;
use lettre::{Message, SmtpTransport, Transport};
use std::env;
use std::thread;
use std::time::Duration;

mod track;

fn main() {
    let creds = Credentials::from_env().unwrap();
    let spotify = ClientCredsSpotify::new(creds);
    spotify.request_token().unwrap();

    let recipient_addresses = &[String::from("alex@das.li"), String::from("scott.mac.32@gmail.com")];
    let playlist_id_string = "6XB4xIqNs20vEGubRKvnAv";
    let polling_interval = 60;

    let playlist_id = PlaylistId::from_id(playlist_id_string)
        .expect("Invalid ID for playlist");
    let mut prev_playlist = spotify.playlist(playlist_id.clone(), None, None)
        .expect("Cannot get initial playlist");
    let mut prev_items = get_tracks(&spotify, &playlist_id);

    thread::sleep(Duration::from_secs(polling_interval));
    loop {
        println!("{} tracks", prev_items.len());
        if let Ok(curr_playlist) = spotify.playlist(playlist_id.clone(), None, None) {
            if curr_playlist.snapshot_id != prev_playlist.snapshot_id {
                let curr_items = get_tracks(&spotify, &playlist_id);
                let spans = compare_playlists(&curr_items, &prev_items);
                for s in spans {
                    match s.tag {
                        Insert => {
                            for (index, item) in &curr_items.iter().enumerate().collect::<Vec<_>>()[s.b_start .. s.b_end] {
                                if let Some(t) = item.track.as_ref().and_then(from_track) {
                                    for address in recipient_addresses {
                                        email_notification(
                                            address,
                                            match &t.preview_url {
                                                Some(link) => format!("{} added a track at position {}:\n{}", user_name(&item.added_by), index + 1, link),
                                                None => format!("{} added {} at position {}.", user_name(&item.added_by), artist_track_name(&t), index + 1),
                                            }
                                        )
                                    }
                                }
                            }
                        }
                        Delete => {
                            for (index, item) in &prev_items.iter().enumerate().collect::<Vec<_>>()[s.a_start .. s.a_end] {
                                if let Some(t) = item.track.as_ref().and_then(from_track) {
                                    for address in recipient_addresses {
                                        email_notification(
                                            address,
                                            match &t.preview_url {
                                                Some(link) => format!("Someone removed a track from position {}, originally added by {}:\n{}", index + 1, user_name(&item.added_by), link),
                                                None => format!("Someone removed {} from position {}, originally added by {}.", artist_track_name(&t), index + 1, user_name(&item.added_by)),
                                            }
                                        )
                                    }
                                }
                            }
                        }
                        Replace => {
                            let curr_block = &curr_items.iter().enumerate().collect::<Vec<_>>()[s.b_start .. s.b_end];
                            let prev_block = &prev_items.iter().enumerate().collect::<Vec<_>>()[s.a_start .. s.a_end];
                            assert!(curr_block.len() == prev_block.len());
                            let zipped = curr_block.iter().zip(prev_block.iter()).collect::<Vec<_>>();
                            for ((index, new), (_, old)) in zipped {
                                for address in recipient_addresses {
                                    if let Some(new_track) = new.track.as_ref().and_then(from_track) {
                                        email_notification(
                                            address,
                                            if let Some(old_track) = old.track.as_ref().and_then(from_track) {
                                                match (&new_track.preview_url, &old_track.preview_url) {
                                                    (Some(new_link), Some(old_link)) => format!("{} replaced a track, originally added by {}, at position {}:\n\nNew track:\n{}\n\nOld track:\n{}", user_name(&new.added_by), user_name(&old.added_by), index + 1, new_link, old_link),
                                                    _ => format!("{} replaced a track, originally added by {}, at position {}:\n\nNew track:\n{}\n\nOld track:\n{}", user_name(&new.added_by), user_name(&old.added_by), index + 1, artist_track_name(&new_track), artist_track_name(&old_track)),
                                                }
                                            }
                                            else {
                                                format!("{} added {} at position {}", user_name(&new.added_by), new_track.name, index + 1)
                                            }
                                        );
                                    }
                                    else {
                                        if let Some(old_track) = old.track.as_ref().and_then(from_track) {
                                            email_notification(
                                                address,
                                                format!("Someone removed {} at position {}, originally added by {}", old_track.name, index + 1, user_name(&old.added_by))
                                            );
                                        }
                                    }
                                }
                            }
                        },
                        _ => continue,
                    }
                }
                prev_playlist = curr_playlist;
                prev_items = curr_items;
            }
        }
        thread::sleep(Duration::from_secs(polling_interval));
    }
}

fn compare_playlists(curr_playlist: &Vec<PlaylistItem>, prev_playlist: &Vec<PlaylistItem>) -> Vec<Span> {
    let curr_tracks: Vec<Option<TrackId<'_>>> = curr_playlist.iter().map(|t| t.track.as_ref().and_then(from_track).map(|t| t.id.clone().unwrap())).collect();
    let prev_tracks: Vec<Option<TrackId<'_>>> = prev_playlist.iter().map(|t| t.track.as_ref().and_then(from_track).map(|t| t.id.clone().unwrap())).collect();
    let differ = Differ::new(&prev_tracks, &curr_tracks);
    return differ.spans();
}

fn from_track(item: &PlayableItem) -> Option<&FullTrack> {
    match item {
        Track(t) => Some(t),
        _ => None
    }
}

fn get_tracks(spotify: &ClientCredsSpotify, playlist_id: &PlaylistId) -> Vec<PlaylistItem> {
    let mut items = Vec::new();
    let mut offset = 0;
    loop {
        let page = spotify.playlist_items_manual(playlist_id.to_owned(), None, None, None, Some(offset)).expect("Error fetching playlist items");
        offset += page.items.len() as u32;
        items.extend(page.items);

        if page.next.is_none() {
            break;
        }
    }
    return items;
}

fn user_name(maybe_user: &Option<PublicUser>) -> String {
    match maybe_user {
        Some(user) => user.id.to_string(),
        None => String::from("Someone")
    }
}

fn artist_track_name(t: &FullTrack) -> String {
    return format!("{} - \"{}\"", t.artists.iter().map(|a| a.clone().name).collect::<Vec<_>>().concat(), t.name);
}

fn email_notification(recipient: &String, message: String) {
    let email = Message::builder()
    .from("Listplay <d.alex.stuart@gmail.com>".parse().unwrap())
    .to(recipient.parse().unwrap())
    .subject("Playlist updated")
    .header(ContentType::TEXT_PLAIN)
    .body(message.clone())
    .unwrap();

    let username = env::var("GMAIL_USERNAME").unwrap();
    let password = env::var("GMAIL_PASSWORD").unwrap();
    let creds = LettreCredentials::new(username, password);

    // Open a remote connection to gmail
    let mailer = SmtpTransport::starttls_relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    // Send the email
    match mailer.send(&email) {
        Ok(_) => println!("Sent email: {}", message),
        Err(e) => panic!("Could not send email: {e:?}"),
    }
}
