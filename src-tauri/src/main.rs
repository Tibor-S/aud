// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod utils;

use cpal::traits::StreamTrait;
use shazamrs::from_file;
use std::{
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use tauri::{State, Window};

use crate::utils::{audio_stream, audio_to_file, default_host};

struct AudioBufferState(Arc<Mutex<Vec<f32>>>); // Ring buffer
struct DeviceNameState(Arc<Mutex<Option<String>>>);
struct StreamReceiverState(Arc<Mutex<Receiver<StreamCommand>>>);
struct StreamSenderState(Arc<Mutex<Sender<StreamCommand>>>);

#[derive(Debug, Clone, PartialEq)]
enum StreamCommand {
    Stop,
}

fn handle_error<E>(e: E) -> RustError
where
    E: std::error::Error,
{
    let formatted = format!("{:?}", e);
    log::error!("{}", formatted);
    RustError::Error { msg: formatted }
}

#[tauri::command]
fn emit_signal(window: Window, audio_buffer_state: State<AudioBufferState>) -> () {
    let audio_buffer = audio_buffer_state.0.lock().unwrap();
    let ret = audio_buffer.clone();
    window.emit("signal", ret).unwrap();
}

#[tauri::command]
async fn init_audio_capture(
    stream_receiver_state: State<'_, StreamReceiverState>,
    audio_buffer_state: State<'_, AudioBufferState>,
    device_name_state: State<'_, DeviceNameState>,
) -> RustResult<()> {
    log::info!("init_audio_capture");
    let stream_receiver = stream_receiver_state.0.clone();
    let audio_buffer = audio_buffer_state.0.clone();
    let device_name = device_name_state.0.clone();
    log::info!(
        "init_audio_capture :: stream_receiver: {:?}",
        stream_receiver
    );
    log::info!(
        "init_audio_capture :: audio_buffer length: {:?}",
        audio_buffer.lock().unwrap().len()
    );
    log::info!(
        "init_audio_capture :: device_name: {:?}",
        *device_name.lock().unwrap()
    );

    let callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        let mut audio_buffer = audio_buffer.lock().unwrap();
        // let max_length = audio_buffer.len();
        let data_length = data.len();
        // let remove_length = data_length.min(max_length);
        audio_buffer.extend(data);
        audio_buffer.drain(0..data_length);
    };

    let device_name = (*device_name.lock().unwrap()).clone();

    let stream = audio_stream(device_name, callback).map_err(handle_error)?;
    log::info!("init_audio_capture :: stream.play()");

    log::info!("init_audio_capture :: stream_receiver recv for Stop");
    while stream_receiver.lock().unwrap().recv() != Ok(StreamCommand::Stop) {}
    stream.pause().map_err(|e| RustError::Error {
        msg: format!("{:?}", e),
    })?;
    log::info!("init_audio_capture :: stream_receiver received Stop");

    drop(stream);
    log::info!("init_audio_capture :: stream dropped");

    log::info!("init_audio_capture :: OK");
    Ok(())
}

#[tauri::command]
fn query_devices() -> RustResult<Vec<String>> {
    log::info!("query_devices");
    let host = default_host();
    log::info!("query_devices :: host: {:?}", host.id());

    utils::query_devices(&host).map_err(handle_error)
}

#[tauri::command]
fn change_device(name: String, device_name_state: State<DeviceNameState>) -> RustResult<()> {
    log::info!("change_device");
    log::info!("change_device :: name: {}", name);
    let device_name = device_name_state.0.clone();
    let host = default_host();
    log::info!("change_device :: host: {:?}", host.id());

    if utils::is_device(&host, &*name).map_err(handle_error)? {
        *device_name.lock().unwrap() = Some(name.clone());
        log::info!("change_device :: changed to {}", name);
    } else {
        log::info!("change_device :: device {} not found", name);
    }

    log::info!("change_device :: OK");

    Ok(())
}

#[tauri::command]
fn set_resolution(
    audio_buffer_state: State<AudioBufferState>,
    resolution: usize,
) -> RustResult<()> {
    log::info!("set_resolution");
    log::info!("set_resolution :: resolution: {}", resolution);
    let audio_buffer = audio_buffer_state.0.clone();
    audio_buffer.lock().unwrap().resize(resolution, 0f32);
    log::info!("set_resolution :: OK");
    Ok(())
}

#[tauri::command]
fn resolution(audio_buffer_state: State<AudioBufferState>) -> RustResult<usize> {
    log::info!("resolution");
    let audio_buffer = audio_buffer_state.0.clone();
    let resolution = audio_buffer.lock().unwrap().len();
    log::info!("resolution :: resolution: {}", resolution);
    Ok(resolution)
}

#[tauri::command]
fn current_device(device_name_state: State<DeviceNameState>) -> String {
    log::info!("current_device");
    let device_name = device_name_state.0.clone();
    let guard = device_name.lock().unwrap();
    (*guard).clone().unwrap_or("Default".into())
}

#[tauri::command]
async fn stop_stream(stream_sender_state: State<'_, StreamSenderState>) -> RustResult<()> {
    log::info!("stop_stream");
    let stream_sender = stream_sender_state.0.clone();
    log::info!("stop_stream :: stream_sender send Stop");
    stream_sender
        .lock()
        .unwrap()
        .send(StreamCommand::Stop)
        .map_err(handle_error)?;
    thread::sleep(Duration::from_secs(1));
    log::info!("stop_stream :: OK");
    Ok(())
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct Art {
    background: Option<String>,
    coverart: Option<String>,
    coverarthq: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct Track {
    pub art: Art,
    pub artist: Option<String>,
    pub track: Option<String>,
    pub album: Option<String>,
}

#[tauri::command]
async fn recognize(device_name_state: State<'_, DeviceNameState>) -> RustResult<Track> {
    log::info!("recognize");
    let device_name = device_name_state.0.clone();
    let device_name = (*device_name.lock().unwrap()).clone();
    log::info!("recognize :: device_name: {:?}", device_name);
    let file = tempfile::NamedTempFile::new().map_err(|e| RustError::Error {
        msg: format!("{:?}", e),
    })?;
    let path = file.path();
    audio_to_file(device_name, path)
        .await
        .map_err(handle_error)?;
    let path_str = path
        .as_os_str()
        .to_str()
        .ok_or(handle_error(RustError::NotUTF8Error))?;
    log::info!("recognize :: path_str: {}", path_str);
    let trck = from_file(path_str).map_err(handle_error)?;

    let obj = trck.as_object();
    let track_obj = obj
        .and_then(|obj| obj.get("track"))
        .and_then(|track| track.as_object());
    let images = track_obj
        .and_then(|track| track.get("images"))
        .and_then(|images| images.as_object());
    let background = images
        .and_then(|images| images.get("background"))
        .and_then(|bkgrnd| bkgrnd.as_str())
        .map(|bkgrnd| bkgrnd.to_owned());
    let coverart = images
        .and_then(|images| images.get("coverart"))
        .and_then(|cvrrt| cvrrt.as_str())
        .map(|bkgrnd| bkgrnd.to_owned());
    let coverarthq = images
        .and_then(|images| images.get("coverarthq"))
        .and_then(|cvrrthq| cvrrthq.as_str())
        .map(|cvrrthq| cvrrthq.to_owned());
    let artist = track_obj
        .and_then(|track| track.get("subtitle"))
        .and_then(|sbttl| sbttl.as_str())
        .map(|sbttl| sbttl.to_owned());
    let track = track_obj
        .and_then(|track| track.get("title"))
        .and_then(|ttl| ttl.as_str())
        .map(|ttl| ttl.to_owned());
    let sections = track_obj
        .and_then(|track| track.get("sections"))
        .and_then(|sections| sections.as_array());
    let metadata = sections.and_then(|sections| {
        sections
            .iter()
            .find_map(|v| v.get("metadata"))
            .and_then(|metadata| metadata.as_array())
    });
    let album = metadata.and_then(|metadata| {
        metadata
            .iter()
            .find(|el| {
                el.get("title")
                    .and_then(|val| val.as_str())
                    .is_some_and(|val| val == "Album")
            })
            .and_then(|el| {
                el.get("text")
                    .and_then(|val| val.as_str())
                    .map(|val| val.to_owned())
            })
    });
    let ret = Track {
        art: Art {
            background,
            coverart,
            coverarthq,
        },
        artist,
        track,
        album,
    };
    log::info!("recognize :: OK");
    Ok(ret)
}

fn main() {
    env_logger::init();
    let audio_buffer = Arc::new(Mutex::new(vec![0.0; 1024]));
    let (tx, rx) = std::sync::mpsc::channel::<StreamCommand>();
    let stream_receiver = Arc::new(Mutex::new(rx));
    let stream_sender = Arc::new(Mutex::new(tx));
    let device_name = Arc::new(Mutex::new(None));

    tauri::Builder::default()
        .manage(AudioBufferState(audio_buffer))
        .manage(StreamReceiverState(stream_receiver))
        .manage(StreamSenderState(stream_sender))
        .manage(DeviceNameState(device_name))
        .invoke_handler(tauri::generate_handler![
            emit_signal,
            init_audio_capture,
            stop_stream,
            query_devices,
            change_device,
            current_device,
            set_resolution,
            resolution,
            recognize
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

type RustResult<T> = Result<T, RustError>;

#[derive(Debug, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum RustError {
    #[error("Error: {msg}")]
    Error { msg: String },
    #[error("Arc has multiple strong references")]
    ArcError,
    #[error("Converted &str was not valid utf-8")]
    NotUTF8Error,
}
