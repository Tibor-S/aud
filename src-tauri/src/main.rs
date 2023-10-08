// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod manager;
mod stream;

use cpal::traits::StreamTrait;
use log::debug;
use shazamrs::from_buffer;
use std::sync::{Arc, Mutex};
use tauri::{State, Window};

struct AudioBufferState(Arc<Mutex<Vec<f32>>>);
struct BufferResolutionState(Arc<Mutex<usize>>);
struct BufferMaxLengthState(Arc<Mutex<usize>>);
struct IsStreamingState(Arc<Mutex<bool>>);
struct ReqStreamingState(Arc<Mutex<bool>>);
struct DeviceNameState(Arc<Mutex<Option<String>>>);

#[tauri::command]
fn emit_signal(
    window: Window,
    audio_buffer_state: State<AudioBufferState>,
    buffer_resolution_state: State<BufferResolutionState>,
) -> () {
    let audio_buffer = audio_buffer_state.0.lock().unwrap();
    let buffer_resolution = buffer_resolution_state.0.lock().unwrap();
    let current_len = audio_buffer.len();
    let len = buffer_resolution.min(current_len);
    let ret = Vec::from(&audio_buffer[(current_len - len)..current_len]);
    window.emit("signal", ret).unwrap();
}

#[tauri::command]
async fn init_audio_capture(
    is_streaming_state: State<'_, IsStreamingState>,
    req_streaming_state: State<'_, ReqStreamingState>,
    audio_buffer_state: State<'_, AudioBufferState>,
    buffer_max_length_state: State<'_, BufferMaxLengthState>,
    device_name_state: State<'_, DeviceNameState>,
) -> RustResult<()> {
    log::info!("init_audio_capture");
    let is_streaming = is_streaming_state.0.clone();
    let req_streaming = req_streaming_state.0.clone();
    let audio_buffer = audio_buffer_state.0.clone();
    let buffer_max_length = buffer_max_length_state.0.clone();
    let device_name = device_name_state.0.clone();
    log::info!(
        "init_audio_capture :: is_streaming: {:?}",
        *is_streaming.lock().unwrap()
    );
    log::info!(
        "init_audio_capture :: req_streaming: {:?}",
        *req_streaming.lock().unwrap()
    );
    log::info!(
        "init_audio_capture :: audio_buffer length: {:?}",
        audio_buffer.lock().unwrap().len()
    );
    log::info!(
        "init_audio_capture :: buffer_max_length: {:?}",
        *buffer_max_length.lock().unwrap()
    );
    log::info!(
        "init_audio_capture :: device_name: {:?}",
        *device_name.lock().unwrap()
    );

    if *is_streaming.lock().unwrap() {
        log::warn!("init_audio_capture :: Already streaming");
        return Err(RustError::Error {
            msg: "Already streaming".into(),
        });
    }

    let host = cpal::default_host();
    let device = manager::device(&host, (*device_name.lock().unwrap()).clone()).map_err(|e| {
        RustError::Error {
            msg: format!("{:?}", e),
        }
    })?;
    let callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        let buffer_max_length = *buffer_max_length.lock().unwrap();
        let mut audio_buffer = audio_buffer.lock().unwrap();
        let current_length = audio_buffer.len();
        let data_length = data.len();
        let new_length = current_length + data_length;
        let si = 0.max((data_length as i32) - (buffer_max_length as i32)) as usize;
        let remove_length = (new_length as i32 - buffer_max_length as i32)
            .max(0)
            .min(current_length as i32) as usize;
        audio_buffer.drain(0..remove_length);
        let new_data = &data[si..data_length];
        audio_buffer.extend(new_data);
    };

    *is_streaming.lock().unwrap() = true;
    log::info!("init_audio_capture :: is_streaming set to true");
    *req_streaming.lock().unwrap() = true;
    log::info!("init_audio_capture :: req_streaming set to true");

    let keep_streaming = Arc::new(Mutex::new(true));
    let thread_keep_streaming = keep_streaming.clone();
    let handle: std::thread::JoinHandle<RustResult<()>> = std::thread::spawn(move || {
        let stream = stream::build(device, callback).map_err(|e| RustError::Error {
            msg: format!("{:?}", e),
        })?;
        stream.play().map_err(|e| RustError::Error {
            msg: format!("{:?}", e),
        })?;
        while *thread_keep_streaming.lock().unwrap() {
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
        drop(stream);
        debug!("stream dropped");
        Ok(())
    });
    log::info!("init_audio_capture :: waiting for req_streaming to be false");
    while *req_streaming.lock().unwrap() {
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
    *keep_streaming.lock().unwrap() = false;

    handle.join().map_err(|e| RustError::Error {
        msg: format!("{:?}", e),
    })??;
    *is_streaming.lock().unwrap() = false;
    log::info!("init_audio_capture :: is_streaming set to false");
    log::info!("init_audio_capture :: OK");
    Ok(())
}

#[tauri::command]
fn query_devices() -> RustResult<Vec<String>> {
    let host = cpal::default_host();
    manager::query_devices(&host).map_err(|e| RustError::Error {
        msg: format!("{:?}", e),
    })
}

#[tauri::command]
fn change_device(name: String, device_name_state: State<DeviceNameState>) -> RustResult<()> {
    log::info!("change_device :: name: {}", name);
    let device_name = device_name_state.0.clone();
    let host = cpal::default_host();

    if manager::is_device(&host, &*name).map_err(|e| RustError::Error {
        msg: format!("{:?}", e),
    })? {
        *device_name.lock().unwrap() = Some(name.clone());
    }

    log::info!("change_device :: OK");

    Ok(())
}

#[tauri::command]
fn set_resolution(
    buffer_resolution_state: State<BufferResolutionState>,
    resolution: usize,
) -> RustResult<()> {
    log::info!("set_resolution :: resolution: {}", resolution);
    let buffer_resolution = buffer_resolution_state.0.clone();
    *buffer_resolution.lock().unwrap() = resolution;
    log::info!("set_resolution :: OK");
    Ok(())
}

#[tauri::command]
fn resolution(buffer_resolution_state: State<'_, BufferResolutionState>) -> RustResult<usize> {
    let buffer_resolution = buffer_resolution_state.0.clone();
    let resolution = *buffer_resolution.lock().unwrap();
    Ok(resolution)
}

#[tauri::command]
fn current_device(device_name_state: State<'_, DeviceNameState>) -> String {
    let device_name = device_name_state.0.clone();
    let guard = device_name.lock().unwrap();
    (*guard).clone().unwrap_or("Default".into())
}

#[tauri::command]
async fn stop_stream(
    is_streaming_state: State<'_, IsStreamingState>,
    req_streaming_state: State<'_, ReqStreamingState>,
    audio_buffer_state: State<'_, AudioBufferState>,
) -> RustResult<()> {
    log::info!("stop_stream");
    let is_streaming = is_streaming_state.0.clone();
    let req_streaming = req_streaming_state.0.clone();
    let audio_buffer = audio_buffer_state.0.clone();
    *req_streaming.lock().unwrap() = false;
    log::info!("stop_stream :: req_streaming set to false");
    audio_buffer.lock().unwrap().clear();
    log::info!("stop_stream :: audio_buffer cleared");
    log::info!("stop_stream :: waiting for is_streaming to be false");
    while *is_streaming.lock().unwrap() {
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
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
    // let id = Uuid::new_v4();
    // let path = std::env::temp_dir()
    //     .join(id.to_string())
    //     .with_extension("wav");
    return Err(RustError::Error {
        msg: "Not implemented".into(),
    });
    let device_name = device_name_state.0.clone();
    let host = cpal::default_host();
    let device = manager::device(&host, (*device_name.lock().unwrap()).clone()).map_err(|e| {
        RustError::Error {
            msg: format!("{:?}", e),
        }
    })?;
    let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let t1_buffer = buffer.clone();
    let callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        t1_buffer.lock().unwrap().extend(data.iter());
    };

    debug!("init recognize stream");
    let stream = stream::build(device, callback).map_err(|e| RustError::Error {
        msg: format!("{:?}", e),
    })?;
    stream.play().map_err(|e| RustError::Error {
        msg: format!("{:?}", e),
    })?;

    std::thread::sleep(std::time::Duration::from_millis(5000));
    drop(stream);
    debug!("recognize stream dropped");
    let b = buffer.lock().unwrap().clone();
    let trck = from_buffer(b, 44100).map_err(|e| RustError::Error {
        msg: format!("{:?}", e),
    })?;
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
    Ok(ret)
}

fn main() {
    env_logger::init();
    debug!("rust");
    let audio_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let buffer_resolution = Arc::new(Mutex::new(1024usize));
    let buffer_max_length = Arc::new(Mutex::new(44100usize));
    let is_streaming = Arc::new(Mutex::new(false));
    let req_streaming = Arc::new(Mutex::new(false));
    let device_name = Arc::new(Mutex::new(None));

    tauri::Builder::default()
        .manage(AudioBufferState(audio_buffer))
        .manage(BufferResolutionState(buffer_resolution))
        .manage(BufferMaxLengthState(buffer_max_length))
        .manage(IsStreamingState(is_streaming))
        .manage(ReqStreamingState(req_streaming))
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
}
