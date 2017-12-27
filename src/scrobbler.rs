use std::collections::VecDeque;
use std::ops::Drop;
use std::convert::TryFrom;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};
use std::thread::{spawn, JoinHandle};

use tokio_core::reactor::Core;

use lastfm::track::ScrobbleTrack;

use client::{Client, Builder};
use utils::{Error, Result};

// ----------------------------------------------------------------

#[derive(Debug, Hash)]
pub struct Track {
    pub name: String,
    pub artist: String,
    pub duration_sec: u32,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    timestamp_utc: Option<u32>,
}

impl Track {
    pub fn new(name: &str, artist: &str, duration_sec: u32) -> Track {
        Track {
            name: name.to_owned(),
            artist: artist.to_owned(),
            duration_sec: duration_sec,
            album: None,
            album_artist: None,
            track_number: None,
            timestamp_utc: None,
        }
    }

    pub fn album(mut self, album: &str) -> Track {
        self.album = Some(album.to_owned());
        self
    }

    pub fn album_artist(mut self, album_artist: &str) -> Track {
        self.album_artist = Some(album_artist.to_owned());
        self
    }

    pub fn track_number(mut self, track_number: u32) -> Track {
        self.track_number = Some(track_number);
        self
    }
}

impl TryFrom<Track> for ScrobbleTrack {
    type Error = Error;
    fn try_from(value: Track) -> Result<ScrobbleTrack> {
        let ts = value.timestamp_utc.ok_or(Error::build("no scrobble timestamp set"))?;
        let tr = ScrobbleTrack::new(
            value.artist,
            value.name,
            ts,
        );
        tr.
    }
}

// ----------------------------------------------------------------

type Cache = Arc<Mutex<VecDeque<Track>>>;

struct Essentials {
    core: Core,
    client: Client,
    cache: Cache,
}

impl Essentials {
    fn new(client_config: Builder, cache: &Cache) -> Result<Essentials> {
        let core = Core::new().map_err(Error::build)?;
        let builder = client_config.handle(core.handle());

        Ok(Essentials { core: core, client: builder.build()?, cache: cache.clone() })
    }
}

// ----------------------------------------------------------------

enum TimerMessage {
    Play(Track),
    Stop,
    Shutdown,
}

enum ScrobbleMessage {
    Scrobble(Track),
    Shutdown,
}

// ----------------------------------------------------------------

// threading mechanics:
// * main -play(track)-> timer
// * main -stop()-> timer
// * timer -scrobble(track)-> scrobbler

// data:
// main has: arc cache and current
// timer has: current track, remaining count
// scrobbler has: arc cache, essentials

// ----------------------------------------------------------------

pub struct Scrobbler {
    scrobbler: Option<JoinHandle<()>>,
    scrobble: Sender<ScrobbleMessage>,

    timer: Option<JoinHandle<()>>,
    update: Sender<TimerMessage>,
    
    cache: Cache,
}

impl Scrobbler {
    pub fn new() -> Result<Scrobbler> {
        let (scrobble_tx, scrobble_rx) = channel();
        let (timer_tx, timer_rx) = channel();

        let timer = spawn(move || {
            let mut current: Option<Track> = None;
            loop {
                match timer_rx.recv() {
                    Ok(TimerMessage::Play(track)) => {

                    },
                    Ok(TimerMessage::Stop) => {

                    },
                    _ => break,
                }
            }
        });

        let scrobbler = spawn(move || {
            loop {
                match scrobble_rx.recv() {
                    Ok(ScrobbleMessage::Scrobble(track)) => {},
                    _ => break,
                }
            }
        });

        Ok(Scrobbler {
            scrobbler: Some(scrobbler),
            scrobble: scrobble_tx.clone(),
            timer: Some(timer),
            update: timer_tx.clone(),
            cache: Arc::new(Mutex::new(VecDeque::new())),
        })
    }

    pub fn now_playing(&self, track: Option<Track>) {
        // Some (play/resume)
        // if played track is different from current (new play):
        // * save the track to current (not scrobbled)
        // * start scrobble timer
        // if played track is the same as current (resume):
        // * if not yet scrobbled, resume scrobble timer
        // * if scrobbled, do nothing

        // None (stop/pause)
        // if current track is not scrobbled, pausescrobble timer
        // if current track is scrobbled, do nothing
    }

    // on_scrobble_timer()
    // * push current track to scrobbler cache
    // * submit scrobble cache
    // * if succeeded, remove all ok items from the cache
    // * if failed:
    //      * if error can be handled (like re-auth), do it
    //      * if network fail, leave the cache as is
    //      * if non-recoverable API fail, retire corresponding cache record(s)
}

impl Drop for Scrobbler {
    fn drop(&mut self) {
        self.scrobble.send(ScrobbleMessage::Shutdown);
        self.update.send(TimerMessage::Shutdown);

        self.scrobbler.take().and_then(|h| h.join().ok());
        self.timer.take().and_then(|h| h.join().ok());
    }
}
