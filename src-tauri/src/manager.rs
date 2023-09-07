use cpal::traits::{DeviceTrait, HostTrait};
use log::{debug, warn};

#[derive(Clone, Debug)]
pub struct Manager {
    buffer_max: usize,
    device_name: Option<String>,
    req_streaming: bool,
    is_streaming: bool,
}

impl Manager {
    pub fn new() -> Self {
        Manager {
            buffer_max: 1024,
            device_name: None,
            req_streaming: false,
            is_streaming: false,
        }
    }

    pub fn query_devices(&self, host: cpal::Host) -> ManagerResult<Vec<String>> {
        let ds = host.input_devices()?;

        let mut names = vec!["Default".into()];
        for d in ds {
            match d.name() {
                Ok(name) => names.push(name),
                Err(e) => return Err(ManagerError::DeviceNameError(e)),
            }
        }

        Ok(names)
    }

    pub fn change_device(&mut self, host: cpal::Host, name: &str) -> ManagerResult<String> {
        debug!("Change device, host: {:?}, name: {:?}", host.id(), name);
        if name == "Default" {
            self.device_name = None;
            return Ok("Default".into());
        }

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

            return Ok(name.to_string());
        }

        Err(ManagerError::DeviceNotFound)
    }

    pub fn device_name(&self) -> Option<String> {
        self.device_name.clone()
    }

    pub fn device(&self, host: &cpal::Host) -> ManagerResult<cpal::Device> {
        let mut p_device: Option<cpal::Device> = None;
        if let Some(name) = self.device_name.clone() {
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
                .ok_or(ManagerError::NoDeviceAvailable)?,
        };

        Ok(device)
    }

    pub fn req_stop(&mut self) {
        self.req_streaming = false;
    }

    pub fn req_start(&mut self) {
        self.req_streaming = true;
    }

    pub fn req_is(&mut self) -> bool {
        self.req_streaming
    }

    pub fn set_streaming(&mut self, is_streaming: bool) {
        self.is_streaming = is_streaming;
    }

    pub fn is_streaming(&mut self) -> bool {
        self.is_streaming
    }

    pub fn resolution(&self) -> usize {
        self.buffer_max
    }

    pub fn set_resolution(&mut self, resolution: usize) {
        self.buffer_max = resolution;
    }
}

pub type ManagerResult<T> = Result<T, ManagerError>;

#[derive(thiserror::Error, Debug)]
pub enum ManagerError {
    #[error("No device was availble")]
    NoDeviceAvailable,
    #[error("No device with the given name found")]
    DeviceNotFound,
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
