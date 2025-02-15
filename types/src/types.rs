use std::time::Duration;

use local_ip_addr::get_local_ip_address;
use lofty::{Accessor, FileProperties, ItemKey, Tag};
use mdns_sd::ServiceInfo;
use music_player_discovery::{SERVICE_NAME, XBMC_SERVICE_NAME};
use music_player_settings::{read_settings, Settings};
use tantivy::{
    schema::{Schema, SchemaBuilder, STORED, STRING, TEXT},
    Document,
};
use upnp_client::types::Metadata;
use url::Url;

pub const CHROMECAST_SERVICE_NAME: &str = "_googlecast._tcp.local.";
pub const AIRPLAY_SERVICE_NAME: &str = "_raop._tcp.local.";

pub const AIRPLAY_DEVICE: &str = "AirPlay";
pub const CHROMECAST_DEVICE: &str = "Chromecast";
pub const XBMC_DEVICE: &str = "XBMC";
pub const MUSIC_PLAYER_DEVICE: &str = "MusicPlayer";
pub const UPNP_DLNA_DEVICE: &str = "UPnP/DLNA";

#[derive(Debug, Clone, Default)]
pub struct Playback {
    pub current_track: Option<Track>,
    pub index: u32,
    pub current_item_id: Option<i32>,
    pub position_ms: u32,
    pub is_playing: bool,
    pub items: Vec<(Track, i32)>,
}

pub struct CurrentPlayback {
    pub current: Option<Playback>,
}

impl CurrentPlayback {
    pub fn new() -> Self {
        Self { current: None }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Song {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub year: Option<u32>,
    pub track: Option<u32>,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub channels: Option<u8>,
    pub duration: Duration,
    pub uri: Option<String>,
    pub cover: Option<String>,
    pub album_artist: String,
}

#[derive(Debug, Clone, Default)]
pub struct SimplifiedSong {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub duration: Duration,
    pub cover: Option<String>,
    pub artist_id: String,
    pub album_id: String,
}

#[derive(Debug, Clone, Default)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artist_id: Option<String>,
    pub year: Option<u32>,
    pub cover: Option<String>,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, Default)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub picture: Option<String>,
    pub albums: Vec<Album>,
    pub songs: Vec<Track>,
}

impl From<Document> for Album {
    fn from(doc: Document) -> Self {
        let mut schema_builder: SchemaBuilder = Schema::builder();

        let id_field = schema_builder.add_text_field("id", STRING | STORED);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let artist_field = schema_builder.add_text_field("artist", TEXT | STORED);
        let year_field = schema_builder.add_i64_field("year", STORED);
        let cover_field = schema_builder.add_text_field("cover", STRING | STORED);

        let id = doc
            .get_first(id_field)
            .unwrap()
            .as_text()
            .unwrap()
            .to_string();
        let title = doc
            .get_first(title_field)
            .unwrap()
            .as_text()
            .unwrap()
            .to_string();
        let artist = doc
            .get_first(artist_field)
            .unwrap()
            .as_text()
            .unwrap()
            .to_string();
        let year = Some(doc.get_first(year_field).unwrap().as_i64().unwrap() as u32);
        let cover = match doc.get_first(cover_field) {
            Some(cover) => cover.as_text(),
            None => None,
        };
        let cover = match cover {
            Some("") => None,
            Some(cover) => Some(cover.to_string()),
            None => None,
        };

        Self {
            id,
            title,
            artist,
            year,
            cover,
            ..Default::default()
        }
    }
}

impl From<Document> for Artist {
    fn from(doc: Document) -> Self {
        let mut schema_builder: SchemaBuilder = Schema::builder();

        let id_field = schema_builder.add_text_field("id", TEXT | STORED);
        let name_field = schema_builder.add_text_field("name", TEXT | STORED);

        let id = doc
            .get_first(id_field)
            .unwrap()
            .as_text()
            .unwrap()
            .to_string();
        let name = doc
            .get_first(name_field)
            .unwrap()
            .as_text()
            .unwrap()
            .to_string();

        Self {
            id,
            name,
            ..Default::default()
        }
    }
}

impl From<Document> for SimplifiedSong {
    fn from(doc: Document) -> Self {
        let mut schema_builder: SchemaBuilder = Schema::builder();

        let id_field = schema_builder.add_text_field("id", STRING | STORED);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let artist_field = schema_builder.add_text_field("artist", TEXT | STORED);
        let album_field = schema_builder.add_text_field("album", TEXT | STORED);
        let genre_field = schema_builder.add_text_field("genre", TEXT);
        let cover_field = schema_builder.add_text_field("cover", STRING | STORED);
        let duration_field = schema_builder.add_i64_field("duration", STORED);
        let artist_id_field = schema_builder.add_text_field("artist_id", STRING | STORED);
        let album_id_field = schema_builder.add_text_field("album_id", STRING | STORED);

        let id = doc
            .get_first(id_field)
            .unwrap()
            .as_text()
            .unwrap()
            .to_string();

        let title = match doc.get_first(title_field) {
            Some(title) => title.as_text().unwrap().to_string(),
            None => String::from(""),
        };
        let artist = match doc.get_first(artist_field) {
            Some(artist) => artist.as_text().unwrap().to_string(),
            None => String::from(""),
        };
        let album = match doc.get_first(album_field) {
            Some(album) => album.as_text().unwrap().to_string(),
            None => String::from(""),
        };
        let genre = match doc.get_first(genre_field) {
            Some(genre) => genre.as_text().unwrap().to_string(),
            None => String::from(""),
        };
        let duration = match doc.get_first(duration_field) {
            Some(duration) => Duration::from_secs(duration.as_i64().unwrap_or_default() as u64),
            None => Duration::from_secs(0),
        };
        let cover = match doc.get_first(cover_field) {
            Some(cover) => cover.as_text(),
            None => None,
        };
        let cover = match cover {
            Some("") => None,
            Some(cover) => Some(cover.to_string()),
            None => None,
        };
        let artist_id = doc
            .get_first(artist_id_field)
            .unwrap()
            .as_text()
            .unwrap()
            .to_string();
        let album_id = doc
            .get_first(album_id_field)
            .unwrap()
            .as_text()
            .unwrap()
            .to_string();
        Self {
            id,
            title,
            artist,
            album,
            genre,
            duration,
            cover,
            artist_id,
            album_id,
            ..Default::default()
        }
    }
}

impl From<&Tag> for Song {
    fn from(tag: &Tag) -> Self {
        Self {
            title: tag.title().unwrap_or("None").to_string(),
            artist: tag.artist().unwrap_or("None").to_string(),
            album: tag.album().unwrap_or("None").to_string(),
            genre: tag.genre().unwrap_or("None").to_string(),
            year: tag.year(),
            track: tag.track(),
            album_artist: tag
                .get_string(&ItemKey::AlbumArtist)
                .unwrap_or(tag.artist().unwrap_or("None"))
                .to_string(),
            ..Default::default()
        }
    }
}

impl From<&Tag> for Artist {
    fn from(tag: &Tag) -> Self {
        let id = format!(
            "{:x}",
            md5::compute(
                tag.get_string(&ItemKey::AlbumArtist)
                    .unwrap_or(tag.artist().unwrap_or("None"))
                    .to_string()
            )
        );
        Self {
            id,
            name: tag
                .get_string(&ItemKey::AlbumArtist)
                .unwrap_or(tag.artist().unwrap_or("None"))
                .to_string(),
            ..Default::default()
        }
    }
}

impl From<&Tag> for Album {
    fn from(tag: &Tag) -> Self {
        let id = format!(
            "{:x}",
            md5::compute(tag.album().unwrap_or("None").to_string())
        );
        let artist_id = Some(format!(
            "{:x}",
            md5::compute(
                tag.get_string(&ItemKey::AlbumArtist)
                    .unwrap_or(tag.artist().unwrap_or("None"))
                    .to_string()
            )
        ));
        Self {
            id,
            title: tag.album().unwrap_or("None").to_string(),
            artist: tag
                .get_string(&ItemKey::AlbumArtist)
                .unwrap_or(tag.artist().unwrap_or("None"))
                .to_string(),
            year: tag.year(),
            artist_id,
            ..Default::default()
        }
    }
}

impl Song {
    pub fn with_properties(&mut self, properties: &FileProperties) -> Self {
        self.bitrate = properties.audio_bitrate();
        self.sample_rate = properties.sample_rate();
        self.bit_depth = properties.bit_depth();
        self.channels = properties.channels();
        self.duration = properties.duration();
        self.clone()
    }
}

#[derive(Default, Clone)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub host: String,
    pub ip: String,
    pub port: u16,
    pub service: String,
    pub app: String,
    pub is_connected: bool,
    pub base_url: Option<String>,
    pub is_cast_device: bool,
    pub is_source_device: bool,
    pub is_current_device: bool,
}

impl Device {
    pub fn with_base_url(&mut self, base_url: Option<String>) -> Self {
        self.base_url = base_url;
        self.clone()
    }
}

impl From<ServiceInfo> for Device {
    fn from(srv: ServiceInfo) -> Self {
        if srv.get_fullname().contains("xbmc") {
            return Self {
                id: srv.get_fullname().to_owned(),
                name: srv
                    .get_fullname()
                    .replace(XBMC_SERVICE_NAME, "")
                    .replace(".", "")
                    .to_owned(),
                host: srv
                    .get_hostname()
                    .split_at(srv.get_hostname().len() - 1)
                    .0
                    .to_owned(),
                ip: srv.get_addresses().iter().next().unwrap().to_string(),
                port: srv.get_port(),
                service: srv.get_fullname().to_owned(),
                app: "xbmc".to_owned(),
                is_connected: false,
                base_url: None,
                is_cast_device: true,
                is_source_device: true,
                is_current_device: false,
            };
        }

        if srv.get_fullname().contains(SERVICE_NAME) {
            let device_id = srv
                .get_fullname()
                .replace(SERVICE_NAME, "")
                .split("-")
                .collect::<Vec<&str>>()[1]
                .replace(".", "")
                .to_owned();

            let config = read_settings().unwrap();
            let settings = config.try_deserialize::<Settings>().unwrap();

            let is_current_device = device_id == settings.device_id
                && srv.get_fullname().split("-").collect::<Vec<&str>>()[0].to_owned() == "http";

            let mut addresses = srv.get_addresses().iter();
            let mut ip = addresses.next().unwrap().to_string();

            if is_current_device {
                ip = get_local_ip_address().unwrap();
            }

            return Self {
                id: device_id.clone(),
                name: srv
                    .get_properties()
                    .get("device_name")
                    .unwrap_or(&device_id.clone())
                    .to_owned(),
                host: srv
                    .get_hostname()
                    .split_at(srv.get_hostname().len() - 1)
                    .0
                    .to_owned(),
                ip,
                port: srv.get_port(),
                service: srv.get_fullname().split("-").collect::<Vec<&str>>()[0].to_owned(),
                app: "music-player".to_owned(),
                is_connected: false,
                base_url: None,
                is_cast_device: true,
                is_source_device: true,
                is_current_device,
            };
        }

        if srv.get_fullname().contains(CHROMECAST_SERVICE_NAME) {
            return Self {
                id: srv.get_properties().get("id").unwrap().to_owned(),
                name: srv.get_properties().get("fn").unwrap().to_owned(),
                host: srv
                    .get_hostname()
                    .split_at(srv.get_hostname().len() - 1)
                    .0
                    .to_owned(),
                ip: srv.get_addresses().iter().next().unwrap().to_string(),
                port: srv.get_port(),
                service: srv.get_fullname().to_owned(),
                app: "chromecast".to_owned(),
                is_connected: false,
                base_url: None,
                is_cast_device: true,
                is_source_device: false,
                is_current_device: false,
            };
        }

        if srv.get_fullname().contains(AIRPLAY_SERVICE_NAME) {
            let name = srv.get_fullname().split("@").collect::<Vec<&str>>()[1]
                .replace(AIRPLAY_SERVICE_NAME, "")
                .to_owned();
            let name = name.split_at(name.len() - 1).0.to_owned();
            return Self {
                id: srv.get_fullname().to_owned(),
                name,
                host: srv
                    .get_hostname()
                    .split_at(srv.get_hostname().len() - 1)
                    .0
                    .to_owned(),
                ip: srv.get_addresses().iter().next().unwrap().to_string(),
                port: srv.get_port(),
                service: srv.get_fullname().to_owned(),
                app: "airplay".to_owned(),
                is_connected: false,
                base_url: None,
                is_cast_device: true,
                is_source_device: false,
                is_current_device: false,
            };
        }

        Self {
            ..Default::default()
        }
    }
}

impl From<upnp_client::types::Device> for Device {
    fn from(device: upnp_client::types::Device) -> Self {
        let (host, port) = Url::parse(&device.location)
            .map(|url| {
                let host = url.host_str().unwrap();
                let port = url.port().unwrap();
                (host.to_string(), port)
            })
            .unwrap();
        let is_cast_device = device
            .device_type
            .contains("urn:schemas-upnp-org:device:MediaRenderer");
        let is_source_device = device
            .device_type
            .contains("urn:schemas-upnp-org:device:MediaServer");

        Self {
            id: device.udn,
            name: device.friendly_name,
            host: host.clone(),
            ip: host.clone(),
            port,
            service: device.device_type,
            app: "dlna".to_owned(),
            is_connected: false,
            base_url: Some(device.location),
            is_cast_device,
            is_source_device,
            is_current_device: false,
        }
    }
}

pub trait Connected {
    fn is_connected(&self, current: Option<&Device>) -> Self;
}

impl Connected for Device {
    fn is_connected(&self, current: Option<&Device>) -> Self {
        match current {
            Some(current) => Self {
                is_connected: self.id == current.id,
                ..self.clone()
            },
            None => Self {
                is_connected: false,
                ..self.clone()
            },
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub duration: Option<f32>,
    pub disc_number: u32,
    pub track_number: Option<u32>,
    pub uri: String,
    pub artists: Vec<Artist>,
    pub album: Option<Album>,
    pub artist: String,
}

#[derive(Default, Clone)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tracks: Vec<Track>,
}

#[derive(Default, Clone)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub playlists: Vec<Playlist>,
}

impl Into<Metadata> for Track {
    fn into(self) -> Metadata {
        Metadata {
            title: self.title,
            artist: Some(self.artist),
            album: self.album.clone().map(|a| a.title),
            album_art_uri: self.album.map(|a| a.cover.unwrap()),
            ..Default::default()
        }
    }
}

pub trait RemoteTrackUrl {
    fn with_remote_track_url(&self, base_url: &str) -> Self;
}

pub trait RemoteCoverUrl {
    fn with_remote_cover_url(&self, base_url: &str) -> Self;
}

impl RemoteTrackUrl for Track {
    fn with_remote_track_url(&self, base_url: &str) -> Self {
        Self {
            uri: format!("{}/tracks/{}", base_url, self.id),
            ..self.clone()
        }
    }
}

impl RemoteCoverUrl for Track {
    fn with_remote_cover_url(&self, base_url: &str) -> Self {
        Self {
            album: match self.album {
                Some(ref album) => Some(album.with_remote_cover_url(base_url)),
                None => None,
            },
            ..self.clone()
        }
    }
}

impl RemoteCoverUrl for Album {
    fn with_remote_cover_url(&self, base_url: &str) -> Self {
        let cover_url = match self.cover {
            Some(ref cover) => match cover.starts_with("http") {
                true => Some(cover.to_owned()),
                false => Some(format!("{}/covers/{}", base_url, cover)),
            },
            None => None,
        };
        Self {
            cover: cover_url,
            tracks: self
                .tracks
                .iter()
                .map(|track| track.with_remote_cover_url(base_url))
                .collect(),
            ..self.clone()
        }
    }
}

impl RemoteTrackUrl for Album {
    fn with_remote_track_url(&self, base_url: &str) -> Self {
        Self {
            tracks: self
                .tracks
                .iter()
                .map(|track| track.with_remote_track_url(base_url))
                .collect(),
            ..self.clone()
        }
    }
}

impl RemoteCoverUrl for Artist {
    fn with_remote_cover_url(&self, base_url: &str) -> Self {
        Self {
            albums: self
                .albums
                .iter()
                .map(|album| album.with_remote_cover_url(base_url))
                .collect(),
            songs: self
                .songs
                .iter()
                .map(|track| track.with_remote_cover_url(base_url))
                .collect(),
            ..self.clone()
        }
    }
}

impl RemoteTrackUrl for Artist {
    fn with_remote_track_url(&self, base_url: &str) -> Self {
        Self {
            songs: self
                .songs
                .iter()
                .map(|track| track.with_remote_track_url(base_url))
                .collect(),
            ..self.clone()
        }
    }
}

impl RemoteTrackUrl for Playlist {
    fn with_remote_track_url(&self, base_url: &str) -> Self {
        Self {
            tracks: self
                .tracks
                .iter()
                .map(|track| track.with_remote_track_url(base_url))
                .collect(),
            ..self.clone()
        }
    }
}

impl RemoteCoverUrl for Playlist {
    fn with_remote_cover_url(&self, base_url: &str) -> Self {
        Self {
            tracks: self
                .tracks
                .iter()
                .map(|track| track.with_remote_cover_url(base_url))
                .collect(),
            ..self.clone()
        }
    }
}
