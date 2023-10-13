use std::ops::Div;
use std::path::{self, PathBuf};
use std::sync::{Arc, Mutex};

use cpal::{traits::DeviceTrait, SampleRate};
use cpal::{Host, Stream};

use cpal::traits::{HostTrait, StreamTrait};
use log::{debug, warn};

pub fn default_host() -> Host {
    cpal::default_host()
}

pub fn query_devices(host: &cpal::Host) -> Result<Vec<String>, UtilsError> {
    let ds = host.input_devices()?;

    let mut names = vec!["Default".into()];
    for d in ds {
        match d.name() {
            Ok(name) => names.push(name),
            Err(e) => return Err(UtilsError::DeviceNameError(e)),
        }
    }

    Ok(names)
}

pub fn is_device(host: &cpal::Host, name: &str) -> Result<bool, UtilsError> {
    debug!("is device, host: {:?}, name: {:?}", host.id(), name);
    let ds = host.input_devices()?;

    for d in ds {
        let n = match d.name() {
            Ok(name) => name,
            Err(e) => {
                warn!("Failed to get device name, skipping, error: {:?}", e);
                continue;
            }
        };

        if n != name {
            continue;
        }

        return Ok(true);
    }

    Err(UtilsError::DeviceNotFound)
}

pub fn device(host: &cpal::Host, device_name: Option<String>) -> Result<cpal::Device, UtilsError> {
    let mut p_device: Option<cpal::Device> = None;

    if let Some(name) = device_name.clone() {
        if name != "Default" {
            let devices = host.input_devices()?;

            for device in devices {
                if let Ok(n) = device.name() {
                    if n == name {
                        p_device = Some(device);
                        break;
                    }
                }
            }
        }
    }

    let device = match p_device {
        Some(device) => device,
        None => host
            .default_input_device()
            .ok_or(UtilsError::NoDeviceAvailable)?,
    };

    Ok(device)
}

pub fn config(device: &cpal::Device) -> Result<cpal::SupportedStreamConfig, UtilsError> {
    let config_range = device
        .supported_input_configs()?
        .next()
        .ok_or(UtilsError::NoConfigAvailable)?;
    let min_sr = config_range.min_sample_rate().0;
    let max_sr = config_range.max_sample_rate().0;
    let config = config_range.with_sample_rate(SampleRate(min_sr.max(40000).min(max_sr)));
    Ok(config)
}

pub fn build<T>(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    data_callback: T,
) -> Result<cpal::Stream, UtilsError>
where
    T: Fn(&[f32], &cpal::InputCallbackInfo) + Sync + Send + 'static,
{
    let config: cpal::StreamConfig = config.clone().into();
    let num_channels = config.channels as usize;
    log::info!("sample_rate: {}", config.sample_rate.0);
    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], info| {
            let mut f_data = Vec::new();
            let real_len = data.len().div(num_channels);
            for i in 0..real_len {
                let si = i * num_channels;
                let ei = si + num_channels;
                f_data.push(data[si..ei].iter().sum::<f32>() / num_channels as f32);
            }
            data_callback(&f_data, info)
        },
        |err| {
            log::error!("an error occurred on stream: {}", err);
        },
        None,
    )?;

    Ok(stream)
}

pub fn audio_stream(
    device_name: Option<String>,
    callback: impl Fn(&[f32], &cpal::InputCallbackInfo) + Sync + Send + 'static,
) -> Result<Stream, UtilsError> {
    let host = default_host();
    let device = device(&host, device_name)?;
    let config: cpal::SupportedStreamConfig = config(&device)?;
    let stream = build(&device, &config, callback)?;
    stream.play()?;

    Ok(stream)
}

pub async fn audio_to_file<P>(device_name: Option<String>, file_path: P) -> Result<(), UtilsError>
where
    P: AsRef<path::Path>,
{
    let host = default_host();
    let device = device(&host, device_name.clone())?;
    let config = config(&device)?;

    let spec = hound::WavSpec {
        channels: config.channels(),
        sample_rate: config.sample_rate().0,
        bits_per_sample: config.sample_format().sample_size() as u16 * 8,
        sample_format: hound::SampleFormat::Float,
    };
    let writer = hound::WavWriter::create(file_path, spec)?;
    let writer = Arc::new(Mutex::new(writer));
    let thread_writer = writer.clone();

    let callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        let mut writer = thread_writer.lock().unwrap();
        for sample in data {
            writer.write_sample(*sample).unwrap();
        }
    };

    let stream = audio_stream(device_name, callback)?;
    std::thread::sleep(std::time::Duration::from_millis(5000));
    drop(stream);
    let guard = Arc::into_inner(writer)
        .ok_or(UtilsError::ArcError)?
        .into_inner()
        .unwrap();
    guard.finalize()?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum UtilsError {
    #[error("No config was availble")]
    NoConfigAvailable,
    #[error("Arc has multiple strong references")]
    ArcError,
    #[error(transparent)]
    SupportedStreamConfigsError(#[from] cpal::SupportedStreamConfigsError),
    #[error(transparent)]
    BuildStreamError(#[from] cpal::BuildStreamError),
    #[error(transparent)]
    PlayStreamError(#[from] cpal::PlayStreamError),
    #[error("No device was availble")]
    NoDeviceAvailable,
    #[error("No device with the given name found")]
    DeviceNotFound,
    #[error(transparent)]
    DevicesError(#[from] cpal::DevicesError),
    #[error(transparent)]
    DeviceNameError(#[from] cpal::DeviceNameError),
    #[error(transparent)]
    HoundError(#[from] hound::Error),
}
