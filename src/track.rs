use rspotify::model::*;

// pub fn cat_tracks(items: Vec<PlaylistItem>) -> Vec<(usize, FullTrack)> {
pub fn cat_tracks(items: Vec<PlaylistItem>) -> Vec<FullTrack> {
    return items
        .into_iter()
        // .enumerate()
        .filter_map(
            // |(index, item)|
            |item| {
                item.track.and_then(|p| match p {
                    PlayableItem::Track(t) => Some(t),
                    _ => None,
                })
            },
        )
        .collect();
}
