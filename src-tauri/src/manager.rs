use cpal::traits::{DeviceTrait, HostTrait};
use log::{debug, warn};

pub fn query_devices(host: &cpal::Host) -> ManagerResult<Vec<String>> {
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

pub fn is_device(host: &cpal::Host, name: &str) -> ManagerResult<bool> {
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

    Err(ManagerError::DeviceNotFound)
}

pub fn device(host: &cpal::Host, device_name: Option<String>) -> ManagerResult<cpal::Device> {
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
            .ok_or(ManagerError::NoDeviceAvailable)?,
    };

    Ok(device)
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
