// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod manager;
mod stream;

use cpal::traits::StreamTrait;
use log::debug;
use manager::Manager;
use serde::de;
use std::{
    sync::{Arc, Mutex},
    thread,
};
use tauri::{Manager as TManager, State, Window};

struct Signal(Arc<Mutex<Vec<f32>>>);
struct ManagerState(Manager);
struct StreamingState(Mutex<bool>);

#[tauri::command]
fn emit_signal(window: Window, signal: State<Signal>) -> () {
    let signal = signal.0.lock().unwrap();
    let len = signal.len();
    window
        .emit("signal", Vec::from(&signal[0..1024.min(len)]))
        .unwrap();
}

#[tauri::command]
async fn init_audio_capture(
    manager: State<'_, ManagerState>,
    streaming: State<'_, StreamingState>,
    signal: State<'_, Signal>,
) -> AudioCaptureResult<()> {
    debug!("init_audio_capture");

    let host = cpal::default_host();
    let manager = manager.0.clone();
    let signal = signal.0.clone();
    let sig_max = manager.buffer_max;
    let callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        let mut signal = signal.lock().unwrap();
        let signal_length = signal.len();
        let data_length = data.len();
        let new_length = signal_length + data_length;
        let si = 0.max((data_length as i32) - (sig_max as i32)) as usize;
        let remove_length = (new_length as i32 - sig_max as i32).max(0) as usize;
        signal.drain(0..remove_length);
        let new_data = &data[si..data_length];
        signal.extend(new_data);
    };
    let stream =
        stream::build(&manager.clone(), &host, callback).map_err(|e| AudioCaptureError::Error {
            msg: format!("{:?}", e),
        })?;
    stream.play().map_err(|e| AudioCaptureError::Error {
        msg: format!("{:?}", e),
    })?;
    *streaming.0.lock().unwrap() = true;
    while *streaming.0.lock().unwrap() {
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
    drop(stream);

    Ok(())
}

type AudioCaptureResult<T> = Result<T, AudioCaptureError>;

#[derive(Debug, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum AudioCaptureError {
    #[error("Error: {msg}")]
    Error { msg: String },
}

#[tauri::command]
fn stop_stream(streaming: State<StreamingState>) {
    *streaming.0.lock().unwrap() = false;
}

fn main() {
    env_logger::init();
    debug!("rust");
    tauri::Builder::default()
        .setup(|app| {
            app.manage(ManagerState(Manager::new()));
            app.manage(StreamingState(Mutex::new(false)));
            app.manage(Signal(Arc::new(Mutex::new(Vec::<f32>::new()))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            emit_signal,
            init_audio_capture,
            stop_stream
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
