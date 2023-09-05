use std::ops::Div;

use crate::manager::{self};
use cpal::traits::DeviceTrait;

pub fn build<T>(device: cpal::Device, data_callback: T) -> StreamResult<cpal::Stream>
where
    T: Fn(&[f32], &cpal::InputCallbackInfo) + Sync + Send + 'static,
{
    let config = device
        .supported_input_configs()?
        .next()
        .ok_or(StreamError::NoConfigAvailable)?
        .with_max_sample_rate();
    let config: cpal::StreamConfig = config.into();
    let num_channels = config.channels as usize;

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

type StreamResult<T> = Result<T, StreamError>;

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("No config was availble")]
    NoConfigAvailable,
    #[error(transparent)]
    ManagerError(#[from] manager::ManagerError),
    #[error(transparent)]
    SupportedStreamConfigsError(#[from] cpal::SupportedStreamConfigsError),
    #[error(transparent)]
    BuildStreamError(#[from] cpal::BuildStreamError),
    #[error(transparent)]
    PlayStreamError(#[from] cpal::PlayStreamError),
}
