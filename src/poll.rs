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

    let playlist_id = PlaylistId::from_id(playlist_id_string).expect("Invalid ID for playlist");
    let mut prev_playlist = spotify.playlist(playlist_id.clone(), None, None).expect("Cannot get initial playlist");
    thread::sleep(Duration::from_secs(polling_interval));
    loop {
        if let Ok(curr_playlist) = spotify.playlist(playlist_id.clone(), None, None) {
            if let Some(spans) = compare_playlists(&curr_playlist, &prev_playlist) {
                for s in spans {
                    match s.tag {
                        Insert => {
                            for (index, (maybe_user, maybe_track)) in &get_tracks(&curr_playlist).iter().enumerate().collect::<Vec<_>>()[s.b_start .. s.b_end] {
                                if let Some(t) = maybe_track {
                                    for address in recipient_addresses {
                                        email_notification(
                                            address,
                                            match &t.preview_url {
                                                Some(link) => format!("{} added a track at position {}:\n{}", user_name(maybe_user), index + 1, link),
                                                None => format!("{} added {} at position {}.", user_name(maybe_user), artist_track_name(t), index + 1),
                                            }
                                        )
                                    }
                                }
                            }
                        }
                        Delete => {
                            for (index, (maybe_user, maybe_track)) in &get_tracks(&prev_playlist).iter().enumerate().collect::<Vec<_>>()[s.a_start .. s.a_end] {
                                if let Some(t) = maybe_track {
                                    for address in recipient_addresses {
                                        email_notification(
                                            address,
                                            match &t.preview_url {
                                                Some(link) => format!("Someone removed a track from position {}, originally added by {}:\n{}", index + 1, user_name(maybe_user), link),
                                                None => format!("Someone removed {} from position {}, originally added by {}.", artist_track_name(t), index + 1, user_name(maybe_user)),
                                            }
                                        )
                                    }
                                }
                            }
                        }
                        Replace => {
                            let curr_block = &get_tracks(&curr_playlist)[s.b_start .. s.b_end];
                            let prev_block = &get_tracks(&prev_playlist)[s.a_start .. s.a_end];
                            assert!(curr_block.len() == prev_block.len());
                            let zipped = curr_block.iter().zip(prev_block.iter()).enumerate().collect::<Vec<_>>();
                            for (index, (new, old)) in zipped {
                                for address in recipient_addresses {
                                    if let (new_user, Some(new_track)) = new {
                                        email_notification(
                                            address,
                                            if let (old_user, Some(old_track)) = old {
                                                match (&new_track.preview_url, &old_track.preview_url) {
                                                    (Some(new_link), Some(old_link)) => format!("{} replaced a track, originally added by {}, at position {}:\n\nNew track:\n{}\n\nOld track:\n{}", user_name(new_user), user_name(old_user), index + 1, new_link, old_link),
                                                    _ => format!("{} replaced a track, originally added by {}, at position {}:\n\nNew track:\n{}\n\nOld track:\n{}", user_name(new_user), user_name(old_user), index + 1, artist_track_name(new_track), artist_track_name(old_track)),
                                                }
                                            }
                                            else {
                                                format!("{} added {} at position {}", user_name(new_user), new_track.name, index + 1)
                                            }
                                        );
                                    }
                                    else {
                                        if let (old_user, Some(old_track)) = old {
                                            email_notification(
                                                address,
                                                format!("Someone removed {} at position {}, originally added by {}", old_track.name, index + 1, user_name(old_user))
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
            }
        }
        thread::sleep(Duration::from_secs(polling_interval));
    }
}

fn compare_playlists(curr_playlist: &FullPlaylist, prev_playlist: &FullPlaylist) -> Option<Vec<Span>> {
    if curr_playlist.snapshot_id != prev_playlist.snapshot_id {
        let curr_tracks: Vec<Option<TrackId<'_>>> = curr_playlist.tracks.items.iter().map(|t| t.track.as_ref().and_then(from_track).map(|t| t.id.clone().unwrap())).collect();
        let prev_tracks: Vec<Option<TrackId<'_>>> = prev_playlist.tracks.items.iter().map(|t| t.track.as_ref().and_then(from_track).map(|t| t.id.clone().unwrap())).collect();
        let differ = Differ::new(&prev_tracks, &curr_tracks);
        return Some(differ.spans());
    }
    return None;
}

fn get_tracks(playlist: &FullPlaylist) -> Vec<(Option<&PublicUser>, Option<&FullTrack>)> {
    return playlist.tracks.items.iter().map(|t| (t.added_by.as_ref(), t.track.as_ref().and_then(from_track))).collect();
}

fn from_track(item: &PlayableItem) -> Option<&FullTrack> {
    match item {
        Track(t) => Some(t),
        _ => None
    }
}

fn user_name(maybe_user: &Option<&PublicUser>) -> String {
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
