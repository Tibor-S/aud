// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod manager;
mod stream;

use cpal::traits::StreamTrait;
use log::debug;
use manager::Manager;
use shazamrs::from_buffer;
use std::sync::{Arc, Mutex};
use tauri::{Manager as TManager, State, Window};

struct Signal(Arc<Mutex<Vec<f32>>>);
struct ManagerState(Mutex<Manager>);

#[tauri::command]
fn emit_signal(window: Window, signal: State<Signal>, manager: State<ManagerState>) -> () {
    let signal = signal.0.lock().unwrap();
    let len = signal.len();
    let max_len = manager.0.lock().unwrap().resolution();

    window
        .emit("signal", Vec::from(&signal[0..max_len.min(len)]))
        .unwrap();
}

#[tauri::command]
async fn init_audio_capture(
    manager: State<'_, ManagerState>,
    signal: State<'_, Signal>,
) -> RustResult<()> {
    debug!("init_audio_capture");

    if manager.0.lock().unwrap().is_streaming() {
        return Err(RustError::Error {
            msg: "Already streaming".into(),
        });
    }

    let host = cpal::default_host();
    let signal = signal.0.clone();
    let sig_max = manager.0.lock().unwrap().resolution();
    let device = manager
        .0
        .lock()
        .unwrap()
        .device(&host)
        .map_err(|e| RustError::Error {
            msg: format!("{:?}", e),
        })?;
    let callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        let mut signal = signal.lock().unwrap();
        let signal_length = signal.len();
        let data_length = data.len();
        let new_length = signal_length + data_length;
        let si = 0.max((data_length as i32) - (sig_max as i32)) as usize;
        let remove_length = (new_length as i32 - sig_max as i32)
            .max(0)
            .min(signal_length as i32) as usize;
        signal.drain(0..remove_length);
        let new_data = &data[si..data_length];
        signal.extend(new_data);
    };

    manager.0.lock().unwrap().set_streaming(true);
    manager.0.lock().unwrap().req_start();
    let keep_streaming = Arc::new(Mutex::new(true));
    let t_keep_streaming = keep_streaming.clone();
    let handle: std::thread::JoinHandle<RustResult<()>> = std::thread::spawn(move || {
        let stream = stream::build(device, callback).map_err(|e| RustError::Error {
            msg: format!("{:?}", e),
        })?;
        stream.play().map_err(|e| RustError::Error {
            msg: format!("{:?}", e),
        })?;
        while *t_keep_streaming.lock().unwrap() {
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
        drop(stream);
        debug!("stream dropped");
        Ok(())
    });
    debug!("req_is: {}", manager.0.lock().unwrap().req_is());
    while manager.0.lock().unwrap().req_is() {
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
    *keep_streaming.lock().unwrap() = false;

    handle.join().map_err(|e| RustError::Error {
        msg: format!("{:?}", e),
    })??;
    manager.0.lock().unwrap().set_streaming(false);
    Ok(())
}

#[tauri::command]
fn query_devices(manager: State<ManagerState>) -> RustResult<Vec<String>> {
    manager
        .0
        .lock()
        .unwrap()
        .query_devices(cpal::default_host())
        .map_err(|e| RustError::Error {
            msg: format!("{:?}", e),
        })
}

#[tauri::command]
fn change_device(name: String, manager: State<ManagerState>) -> RustResult<()> {
    manager
        .0
        .lock()
        .unwrap()
        .change_device(cpal::default_host(), name.as_str())
        .map_err(|e| RustError::Error {
            msg: format!("{:?}", e),
        })?;
    Ok(())
}

#[tauri::command]
async fn set_resolution(resolution: usize, manager: State<'_, ManagerState>) -> RustResult<()> {
    debug!("{}", manager.0.lock().unwrap().resolution());
    manager.0.lock().unwrap().set_resolution(resolution);
    debug!("{}", manager.0.lock().unwrap().resolution());
    Ok(())
}

#[tauri::command]
async fn resolution(manager: State<'_, ManagerState>) -> RustResult<usize> {
    debug!("{}", manager.0.lock().unwrap().resolution());
    Ok(manager.0.lock().unwrap().resolution())
}

#[tauri::command]
fn current_device(manager: State<ManagerState>) -> String {
    manager
        .0
        .lock()
        .unwrap()
        .device_name()
        .unwrap_or("Default".into())
}

#[tauri::command]
async fn stop_stream(
    manager: State<'_, ManagerState>,
    signal: State<'_, Signal>,
) -> RustResult<()> {
    manager.0.lock().unwrap().req_stop();
    signal.0.lock().unwrap().clear();
    while manager.0.lock().unwrap().is_streaming() {
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
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
async fn recognize(manager: State<'_, ManagerState>) -> RustResult<Track> {
    // let id = Uuid::new_v4();
    // let path = std::env::temp_dir()
    //     .join(id.to_string())
    //     .with_extension("wav");

    let host = cpal::default_host();
    let device = manager
        .0
        .lock()
        .unwrap()
        .device(&host)
        .map_err(|e| RustError::Error {
            msg: format!("{:?}", e),
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

    while buffer.lock().unwrap().len() < 48000 * 5 {
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
    drop(stream);
    debug!("recognize stream dropped");
    let b = buffer.lock().unwrap().clone();
    let trck = from_buffer(b, 48000).map_err(|e| RustError::Error {
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
    tauri::Builder::default()
        .setup(|app| {
            app.manage(ManagerState(Mutex::new(Manager::new())));
            app.manage(Signal(Arc::new(Mutex::new(Vec::<f32>::new()))));
            Ok(())
        })
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
