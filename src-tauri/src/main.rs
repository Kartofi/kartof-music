// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use lofty::mpeg::MpegFile;
use lofty::{
    read_from_path, Accessor, AudioFile, ParseOptions, Picture, TagType, TaggedFile, TaggedFileExt,
};

use rodio::Sink;
use rodio::{source::Source, Decoder, OutputStream, OutputStreamHandle};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, ReadDir};
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{thread, vec};

use tauri::State;

#[derive(Clone, Serialize, Deserialize)]
struct Music {
    path: String,
    properties: Option<Properties>,
    position: u64,
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
            let file = BufReader::new(File::open("audios/Oreo.mp3").unwrap());

            let play = stream_handle.play_once(file).unwrap();
            play.stop();

            let mut sounds_to_play: Vec<Music> = Vec::new();

            let mut playing: Option<Music> = None;

            let mut started_playing = Instant::now();

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
                    playing = None;
                } else if data.action_type == MusicPlayerActions::Resume {
                    play.play();
                } else if data.action_type == MusicPlayerActions::Pause {
                    play.pause();
                } else if data.action_type == MusicPlayerActions::Enqueue {
                    let path = data.data.unwrap();

                    let properties = get_properties(path.clone());

                    sounds_to_play.push(Music {
                        path: path.clone(),
                        properties: Some(properties.clone()),
                        position: 0,
                    });
                } else if data.action_type == MusicPlayerActions::Volume {
                    let volume: f32 = data.data.unwrap().parse().unwrap();
                    play.set_volume(volume);
                }
                if sounds_to_play.len() > 0 && play.empty() && play.is_paused() == false {
                    let file = BufReader::new(File::open(sounds_to_play[0].path.clone()).unwrap());
                    let source = Decoder::new(file).unwrap();

                    play.append(source);

                    play.play();

                    started_playing = Instant::now();

                    playing = Some(sounds_to_play[0].clone());
                    sounds_to_play.remove(0);
                } else if play.is_paused() == false && !play.empty() && playing.is_none() == false {
                    let mut cloning = playing.clone().unwrap();

                    cloning.position = started_playing.elapsed().as_secs();
                    playing = Some(cloning);
                } else {
                    playing = None;
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
    pub fn resume(&self) {
        self.tx
            .send(PlayerAction::new(MusicPlayerActions::Resume, None))
            .unwrap();
        *self.is_playing.lock().unwrap() = true;
    }
    pub fn pause(&self) {
        self.tx
            .send(PlayerAction::new(MusicPlayerActions::Pause, None))
            .unwrap();
        *self.is_playing.lock().unwrap() = false;
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
    year: Option<u32>,
    duration: Option<f64>,
}
fn get_properties(path: String) -> Properties {
    let tagged_file_result = read_from_path(path.clone());
    let tagged_file: Option<TaggedFile> = match tagged_file_result {
        Ok(data) => Some(data),
        Err(err) => None,
    };
    if tagged_file.is_none() {
        return Properties {
            title: None,
            artist: None,
            year: None,
            duration: None,
        };
    }
    let unwraped_tagged_file = tagged_file.unwrap();

    let primary_tag_option: Option<&lofty::Tag> = unwraped_tagged_file.primary_tag();

    let mut properties = Properties {
        title: None,
        artist: None,
        year: None,
        duration: None,
    };
    if primary_tag_option.is_none() == false {
        let primary_tag = primary_tag_option.unwrap();

        let title = primary_tag.title();
        if title.is_none() == false {
            properties.title = Some(title.unwrap().to_string());
        }

        let artist = primary_tag.artist();
        if artist.is_none() == false {
            properties.artist = Some(artist.unwrap().to_string());
        }

        let title = primary_tag.title();
        if title.is_none() == false {
            properties.title = Some(title.unwrap().to_string());
        }

        let year = primary_tag.year();
        if year.is_none() == false {
            properties.year = Some(year.unwrap());
        }

        properties.duration = Some(unwraped_tagged_file.properties().duration().as_secs() as f64);
    }

    properties
}

fn get_cover(path: String) -> Option<Vec<u8>> {
    let tagged_file = read_from_path(path.clone()).unwrap();
    let primary_tag_option: Option<&lofty::Tag> = tagged_file.primary_tag();

    if primary_tag_option.is_none() == false {
        let primary_tag = primary_tag_option.unwrap();
        primary_tag.picture_count();
        let pics: &[Picture] = primary_tag.pictures();
        let first = pics.first();
        if first != None {
            return Some(first.unwrap().data().to_vec());
        }
    }
    None
}

fn get_available_musics(path: &str) -> Vec<Music> {
    let mut result: Vec<Music> = Vec::new();

    match fs::read_dir(path) {
        Ok(data) => {
            data.into_iter().for_each(|item| {
                let path = item.unwrap().path();
                let path_string = path.to_str().unwrap().to_owned();
                println!("{}", path_string);
                if !path_string.ends_with(".geetkeep") {
                    let properties = get_properties(path_string.clone());
                    let music = Music {
                        path: path_string.clone(),
                        properties: Some(properties.clone()),
                        position: 0,
                    };

                    result.push(music);
                }
            });
        }
        Err(err) => println!("{}", err),
    };

    return result;
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
#[tauri::command]
fn get_cover_(path: String) -> Option<Vec<u8>> {
    get_cover(path)
}
#[tauri::command]
fn get_sounds() -> Vec<Music> {
    get_available_musics("audios")
}
#[tauri::command]
fn pause(musicplayer: State<MusicPlayer<String>>) {
    musicplayer.pause();
}
#[tauri::command]
fn resume(musicplayer: State<MusicPlayer<String>>) {
    musicplayer.resume();
}
fn main() {
    let player = MusicPlayer::new();
    tauri::Builder::default()
        .manage(player)
        .invoke_handler(tauri::generate_handler![
            enqueue,
            resume,
            pause,
            get_queue_length,
            get_queue,
            get_playing,
            get_cover_,
            get_sounds,
            set_volume,
            get_volume
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
