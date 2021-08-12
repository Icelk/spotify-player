use rspotify::model::*;

/// Player state
#[derive(Debug)]
pub struct PlayerState {
    pub context_cache: lru::LruCache<String, Context>,

    pub user_playlists: Vec<playlist::SimplifiedPlaylist>,
    pub devices: Vec<device::Device>,
    pub auth_token_expires_at: std::time::SystemTime,
    pub context: Context,
    pub playback: Option<context::CurrentlyPlaybackContext>,
    pub playback_last_updated: Option<std::time::SystemTime>,
}

/// Playing context (album, playlist, etc) of the current track
#[derive(Clone, Debug)]
pub enum Context {
    Playlist(playlist::FullPlaylist, Vec<Track>),
    Album(album::FullAlbum, Vec<Track>),
    Artist(artist::FullArtist, Vec<Track>, Vec<Album>, Vec<Artist>),
    Unknown(String),
}

#[derive(Debug)]
/// Order of sorting tracks in a playing context
pub enum ContextSortOrder {
    AddedAt,
    TrackName,
    Album,
    Artists,
    Duration,
}

#[derive(Default, Debug, Clone)]
/// A simplified version of `rspotify` track
pub struct Track {
    pub id: Option<String>,
    pub uri: String,
    pub name: String,
    pub artists: Vec<Artist>,
    pub album: Album,
    pub duration: u32,
    pub added_at: u64,
}

#[derive(Default, Debug, Clone)]
/// A simplified version of `rspotify` album
pub struct Album {
    pub id: Option<String>,
    pub uri: Option<String>,
    pub name: String,
}

#[derive(Default, Debug, Clone)]
/// A simplified version of `rspotify` artist
pub struct Artist {
    pub id: Option<String>,
    pub uri: Option<String>,
    pub name: String,
}

impl PlayerState {
    /// gets the current playing track
    pub fn get_current_playing_track(&self) -> Option<&track::FullTrack> {
        match self.playback {
            None => None,
            Some(ref playback) => match playback.item {
                Some(rspotify::model::PlayingItem::Track(ref track)) => Some(track),
                _ => None,
            },
        }
    }

    /// gets the current playback progress
    pub fn get_playback_progress(&self) -> Option<u32> {
        match self.playback {
            None => None,
            Some(ref playback) => match playback.item {
                Some(rspotify::model::PlayingItem::Track(ref track)) => {
                    let progress_ms = (playback.progress_ms.unwrap() as u128)
                        + if playback.is_playing {
                            std::time::SystemTime::now()
                                .duration_since(self.playback_last_updated.unwrap())
                                .unwrap()
                                .as_millis()
                        } else {
                            0
                        };
                    if progress_ms > (track.duration_ms as u128) {
                        Some(track.duration_ms)
                    } else {
                        Some(progress_ms as u32)
                    }
                }
                _ => None,
            },
        }
    }
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            context_cache: lru::LruCache::new(64),
            auth_token_expires_at: std::time::SystemTime::now(),
            devices: vec![],
            user_playlists: vec![],
            context: Context::Unknown("".to_owned()),
            playback: None,
            playback_last_updated: None,
        }
    }
}

impl Context {
    /// sorts tracks in the current playing context given a context sort oder
    pub fn sort_tracks(&mut self, sort_oder: ContextSortOrder) {
        let tracks = self.get_tracks_mut();
        if let Some(tracks) = tracks {
            tracks.sort_by(|x, y| sort_oder.compare(x, y));
        }
    }

    /// reverses order of tracks in the current playing context
    pub fn reverse_tracks(&mut self) {
        let tracks = self.get_tracks_mut();
        if let Some(tracks) = tracks {
            tracks.reverse();
        }
    }

    /// gets the description of current playing context
    pub fn get_description(&self) -> String {
        match self {
            Context::Unknown(_) => {
                "Cannot infer the playing context from the current playback".to_owned()
            }
            Context::Album(ref album, _) => {
                format!("Album: {}", album.name)
            }
            Context::Playlist(ref playlist, _) => {
                format!("Playlist: {}", playlist.name)
            }
            Context::Artist(ref artist, _, _, _) => {
                format!("Artist: {}", artist.name)
            }
        }
    }

    /// gets all tracks inside the current playing context (mutable)
    pub fn get_tracks_mut(&mut self) -> Option<&mut Vec<Track>> {
        match self {
            Context::Unknown(_) => None,
            Context::Album(_, ref mut tracks) => Some(tracks),
            Context::Playlist(_, ref mut tracks) => Some(tracks),
            Context::Artist(_, ref mut tracks, _, _) => Some(tracks),
        }
    }

    /// gets current playing context's uri
    pub fn get_uri(&self) -> &str {
        match self {
            Context::Unknown(ref uri) => uri,
            Context::Album(ref album, _) => &album.uri,
            Context::Playlist(ref playlist, _) => &playlist.uri,
            Context::Artist(ref artist, _, _, _) => &artist.uri,
        }
    }
}

impl Track {
    /// gets the track's artists information
    pub fn get_artists_info(&self) -> String {
        self.artists
            .iter()
            .map(|a| a.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// gets the track basic information (track's name, artists' name and album's name)
    pub fn get_basic_info(&self) -> String {
        format!(
            "{} {} {}",
            self.name,
            self.get_artists_info(),
            self.album.name
        )
    }
}

impl std::fmt::Display for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_basic_info())
    }
}

impl std::fmt::Display for Album {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl std::fmt::Display for Artist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<playlist::PlaylistTrack> for Track {
    fn from(t: playlist::PlaylistTrack) -> Self {
        let track = t.track.unwrap();
        Self {
            id: track.id,
            uri: track.uri,
            name: track.name,
            artists: track
                .artists
                .into_iter()
                .map(|a| Artist {
                    id: a.id,
                    uri: a.uri,
                    name: a.name,
                })
                .collect(),
            album: Album {
                id: track.album.id,
                uri: track.album.uri,
                name: track.album.name,
            },
            duration: track.duration_ms,
            added_at: t.added_at.timestamp() as u64,
        }
    }
}

impl From<track::SimplifiedTrack> for Track {
    fn from(track: track::SimplifiedTrack) -> Self {
        Self {
            id: track.id,
            uri: track.uri,
            name: track.name,
            artists: track
                .artists
                .into_iter()
                .map(|a| Artist {
                    id: a.id,
                    uri: a.uri,
                    name: a.name,
                })
                .collect(),
            album: Album::default(),
            duration: track.duration_ms,
            added_at: 0,
        }
    }
}

impl From<track::FullTrack> for Track {
    fn from(track: track::FullTrack) -> Self {
        Self {
            id: track.id,
            uri: track.uri,
            name: track.name,
            artists: track
                .artists
                .into_iter()
                .map(|a| Artist {
                    id: a.id,
                    uri: a.uri,
                    name: a.name,
                })
                .collect(),
            album: Album {
                name: track.album.name,
                id: track.album.id,
                uri: track.album.uri,
            },
            duration: track.duration_ms,
            added_at: 0,
        }
    }
}

impl ContextSortOrder {
    pub fn compare(&self, x: &Track, y: &Track) -> std::cmp::Ordering {
        match *self {
            Self::AddedAt => x.added_at.cmp(&y.added_at),
            Self::TrackName => x.name.cmp(&y.name),
            Self::Album => x.album.name.cmp(&y.album.name),
            Self::Duration => x.duration.cmp(&y.duration),
            Self::Artists => x.get_artists_info().cmp(&y.get_artists_info()),
        }
    }
}