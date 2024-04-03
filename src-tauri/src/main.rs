// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use audiotags::Tag;
use lofty::mpeg::MpegFile;
use lofty::{read_from_path, AudioFile, ParseOptions, TagType};

use rodio::Sink;
use rodio::{source::Source, Decoder, OutputStream, OutputStreamHandle};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{thread, vec};

use tauri::State;

#[derive(Clone, Serialize, Deserialize)]
struct Music {
    title: String,
    properties: Option<Properties>,
    started: Duration,
}
struct MusicPlayer<T> {
    playing: Arc<Mutex<Option<Music>>>,
    is_playing: Mutex<bool>,
    queue: Arc<Mutex<Vec<Music>>>,
    volume: Mutex<f32>,

    tx: Sender<PlayerAction<T>>,
    client_receiver: Mutex<Receiver<String>>,
}
#[derive(PartialEq)]
enum MusicPlayerActions {
    Error = 0,
    Enqueue = 1,
    Stop = 2,
    Pause = 3,
    Resume = 4,
    Volume = 5,
}

struct PlayerAction<T> {
    action_type: MusicPlayerActions,
    data: Option<T>,
}
impl PlayerAction<String> {
    pub fn new(action_type: MusicPlayerActions, data: Option<String>) -> PlayerAction<String> {
        PlayerAction {
            action_type: action_type,
            data: data,
        }
    }
}
impl MusicPlayer<String> {
    pub fn new() -> MusicPlayer<String> {
        let (tx, rx): (Sender<PlayerAction<String>>, Receiver<PlayerAction<String>>) =
            mpsc::channel();

        let (client_sender, client_receiver): (Sender<String>, Receiver<String>) = mpsc::channel();

        let queue_vec: Vec<Music> = Vec::new();
        let queue = Arc::from(Mutex::from(queue_vec));

        let queue_clone = Arc::clone(&queue);

        let playing_data: Option<Music> = None;
        let playing: Arc<Mutex<Option<Music>>> = Arc::from(Mutex::from(playing_data));
        let playing_clone = Arc::clone(&playing);

        thread::spawn(move || {
            let (_stream, stream_handle): (OutputStream, OutputStreamHandle) =
                OutputStream::try_default().unwrap();
            // Load a sound from a file, using a path relative to Cargo.toml
            let file = BufReader::new(File::open("audios/sound.wav").unwrap());

            let play = stream_handle.play_once(file).unwrap();
            play.stop();

            let mut sounds_to_play: Vec<Music> = Vec::new();

            let mut playing: Option<Music> = None;
            loop {
                let data = rx
                    .recv_timeout(Duration::from_millis(10))
                    .unwrap_or(PlayerAction {
                        action_type: MusicPlayerActions::Error,
                        data: None,
                    });
                if data.action_type == MusicPlayerActions::Stop {
                    play.clear();
                    sounds_to_play.clear();
                } else if data.action_type == MusicPlayerActions::Resume {
                    play.play();
                } else if data.action_type == MusicPlayerActions::Enqueue {
                    let path = data.data.unwrap();

                    let properties = get_properties(path.clone());

                    sounds_to_play.push(Music {
                        title: path.clone(),
                        properties: Some(properties),
                        started: Duration::from_secs(0),
                    });
                } else if data.action_type == MusicPlayerActions::Error {
                    if sounds_to_play.len() > 0 && play.empty() {
                        let file =
                            BufReader::new(File::open(sounds_to_play[0].title.clone()).unwrap());
                        let source = Decoder::new(file).unwrap();

                        play.append(source);

                        play.play();

                        sounds_to_play[0].started = get_timestamp();

                        playing = Some(sounds_to_play[0].clone());
                        sounds_to_play.remove(0);
                    }
                } else if data.action_type == MusicPlayerActions::Volume {
                    let volume: f32 = data.data.unwrap().parse().unwrap();
                    play.set_volume(volume);
                }
                if let Ok(mut queue) = queue_clone.lock() {
                    *queue = sounds_to_play.clone();
                }
                if let Ok(mut playing_) = playing_clone.lock() {
                    *playing_ = playing.clone();
                }
            }
        });

        MusicPlayer {
            playing: playing,
            is_playing: Default::default(),
            queue: queue,
            volume: Mutex::from(1.0),

            tx: tx,
            client_receiver: Mutex::from(client_receiver),
        }
    }
    pub fn enqueue(&self, path: String) -> bool {
        if !path_exists(&path) {
            return false;
        }
        self.tx
            .send(PlayerAction::new(MusicPlayerActions::Enqueue, Some(path)))
            .unwrap();
        *self.is_playing.lock().unwrap() = true;
        true
    }
    pub fn stop(&self) {
        self.tx
            .send(PlayerAction::new(MusicPlayerActions::Stop, None))
            .unwrap();
        *self.is_playing.lock().unwrap() = false;
    }
    pub fn get_queue_length(&self) -> usize {
        let length = self.queue.lock().unwrap().len();
        length
    }
    pub fn get_queue(&self) -> Vec<Music> {
        let queue = self.queue.lock().unwrap().to_vec();
        queue
    }
    pub fn get_playing(&self) -> Option<Music> {
        self.playing.lock().unwrap().clone()
    }
    pub fn set_volume(&self, volume: f32) {
        self.tx
            .send(PlayerAction::new(
                MusicPlayerActions::Volume,
                Some(volume.to_string()),
            ))
            .unwrap();
        *self.volume.lock().unwrap() = volume;
    }
    pub fn get_volume(&self) -> f32 {
        *self.volume.lock().unwrap()
    }
}
// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[derive(Clone, Serialize, Deserialize)]
struct Properties {
    title: Option<String>,
    artist: Option<String>,
    year: Option<i32>,
    image: Option<Vec<u8>>,
    duration: Option<f64>,
}
fn get_properties(path: String) -> Properties {
    let mut tag = Tag::new().read_from_path(path.clone()).unwrap();

    let mut properties = Properties {
        title: None,
        artist: None,
        year: tag.year(),
        image: None,
        duration: None,
    };

    if tag.album_cover() != None {
        //properties.image = Some(tag.album_cover().unwrap().data.to_vec());
    }
    if tag.title() != None {
        properties.artist = Some(tag.title().unwrap().to_string());
    }
    if tag.artist() != None {
        properties.artist = Some(tag.artist().unwrap().to_string());
    }
    if tag.duration() == None {
        let tagged_file = read_from_path(path.clone()).unwrap();
        properties.duration = Some(tagged_file.properties().duration().as_secs() as f64);
    } else {
        properties.duration = tag.duration();
    }

    properties
}
fn path_exists(path: &str) -> bool {
    Path::new(path).exists()
}
fn get_timestamp() -> Duration {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    return since_the_epoch;
}
#[tauri::command]
fn enqueue(path: &str, musicplayer: State<MusicPlayer<String>>) -> bool {
    musicplayer.enqueue(path.to_string())
}
#[tauri::command]
fn get_queue_length(musicplayer: State<MusicPlayer<String>>) -> usize {
    musicplayer.get_queue_length()
}
#[tauri::command]
fn get_queue(musicplayer: State<MusicPlayer<String>>) -> Vec<Music> {
    musicplayer.get_queue()
}
#[tauri::command]
fn set_volume(musicplayer: State<MusicPlayer<String>>, volume: f32) {
    musicplayer.set_volume(volume);
}
#[tauri::command]
fn get_volume(musicplayer: State<MusicPlayer<String>>) -> f32 {
    musicplayer.get_volume()
}
#[tauri::command]
fn get_playing(musicplayer: State<MusicPlayer<String>>) -> Option<Music> {
    musicplayer.get_playing()
}
fn main() {
    let player = MusicPlayer::new();
    tauri::Builder::default()
        .manage(player)
        .invoke_handler(tauri::generate_handler![
            enqueue,
            get_queue_length,
            get_queue,
            get_playing,
            set_volume,
            get_volume
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
