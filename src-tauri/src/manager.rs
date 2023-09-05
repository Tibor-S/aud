use std::{
    ops::{Div, Range},
    sync::Mutex,
    thread,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream,
};
use log::{debug, warn};
use serde_json::error;

#[derive(Clone, Debug)]
pub struct Manager {
    pub buffer_max: usize,
    device_name: Option<String>,
}

impl Manager {
    pub fn new() -> Self {
        Manager {
            buffer_max: 1024,
            device_name: None,
        }
    }

    pub fn query_devices(&self, host: cpal::Host) -> ManagerResult<Vec<String>> {
        let ds = host.input_devices()?;

        let mut names = Vec::new();
        for d in ds {
            match d.name() {
                Ok(name) => names.push(name),
                Err(e) => return Err(ManagerError::DeviceNameError(e)),
            }
        }

        Ok(names)
    }

    pub fn change_device(&mut self, host: cpal::Host, name: &str) -> ManagerResult<()> {
        debug!("Change device, host: {:?}, name: {:?}", host.id(), name);
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

            self.device_name = Some(name.to_string());
        }

        Err(ManagerError::DeviceNotFound)
    }

    pub fn device_name(&self) -> Option<String> {
        self.device_name.clone()
    }

    pub fn device(&self, host: &cpal::Host) -> ManagerResult<cpal::Device> {
        let mut p_device: Option<cpal::Device> = None;
        if let Some(name) = self.device_name.clone() {
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

        let device = match p_device {
            Some(device) => device,
            None => host
                .default_input_device()
                .ok_or(ManagerError::NoDeviceAvailable)?,
        };

        Ok(device)
    }
}

pub type ManagerResult<T> = Result<T, ManagerError>;

#[derive(thiserror::Error, Debug)]
pub enum ManagerError {
    #[error("No device was availble")]
    NoDeviceAvailable,
    #[error("No device with the given name found")]
    DeviceNotFound,
    #[error("Found no config supporting the given device")]
    NoConfigSupport,
    #[error(transparent)]
    DevicesError(#[from] cpal::DevicesError),
    #[error(transparent)]
    DeviceNameError(#[from] cpal::DeviceNameError),
    #[error(transparent)]
    SupportedStreamConfigsError(#[from] cpal::SupportedStreamConfigsError),
    #[error(transparent)]
    BuildStreamError(#[from] cpal::BuildStreamError),
    #[error(transparent)]
    PlayStreamError(#[from] cpal::PlayStreamError),
}
