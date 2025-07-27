//! Connect to an LED badge via USB HID

use std::sync::Arc;

use anyhow::{Context, Result};
use hidapi::{DeviceInfo, HidApi, HidDevice};

use crate::protocol::PayloadBuffer;

enum DeviceType {
    // rename if we add another device type
    TheOnlyOneWeSupportForNow,
}

impl DeviceType {
    fn new(info: &DeviceInfo) -> Option<Self> {
        Some(match (info.vendor_id(), info.product_id()) {
            (0x0416, 0x5020) => Self::TheOnlyOneWeSupportForNow,
            _ => return None,
        })
    }
}

/// A discovered USB device
pub struct Device {
    api: Arc<HidApi>,
    info: DeviceInfo,
    type_: DeviceType,
}

impl Device {
    /// Return a list of all usb devies as a string representation
    pub fn list_all() -> Result<Vec<String>> {
        let api = HidApi::new().context("create hid api")?;
        let devices = api.device_list();

        Ok(devices
            .map(|info| {
                format!(
                    "{:?}: vendor_id={:#06x} product_id={:#06x} manufacturer={:?} product={:?}",
                    info.path(),
                    info.vendor_id(),
                    info.product_id(),
                    info.manufacturer_string(),
                    info.product_string(),
                )
            })
            .collect())
    }

    /// Return all supported devices
    pub fn enumerate() -> Result<Vec<Self>> {
        let api = HidApi::new().context("create hid api")?;
        let api = Arc::new(api);

        let devices = api.device_list();
        let devices = devices
            .filter_map(|info| {
                DeviceType::new(info).map(|type_| Device {
                    api: api.clone(),
                    info: info.clone(),
                    type_,
                })
            })
            .collect();

        Ok(devices)
    }

    /// Return the single supported device
    ///
    /// This function returns an error if no device could be found
    /// or if multiple devices would match.
    pub fn single() -> Result<Self> {
        let mut devices = Self::enumerate()?.into_iter();
        let device = devices.next().context("no device found")?;
        anyhow::ensure!(devices.next().is_none(), "multiple devices found");
        Ok(device)
    }

    /// Write a payload to the device
    pub fn write(&self, payload: PayloadBuffer) -> Result<()> {
        let device = self.info.open_device(&self.api).context("open device")?;
        match self.type_ {
            DeviceType::TheOnlyOneWeSupportForNow => {
                write_raw(&device, payload.into_padded_bytes().as_ref())
            }
        }
    }
}

fn write_raw(device: &HidDevice, data: &[u8]) -> Result<()> {
    anyhow::ensure!(data.len() % 64 == 0, "payload not padded to 64 bytes");

    // the device will brick itself if the payload is too long (more then 8192 bytes)
    anyhow::ensure!(data.len() <= 8192, "payload too long (max 8192 bytes)");

    // just to be sure
    assert!(data.len() <= 8192);

    let mut written: usize = 0;

    #[cfg(windows)]
    {
        written = 0;

        while written < data.len() {
            let new_data: &[u8] = &prepend_byte_and_offset(data, written);
            let n = device.write(new_data).context("write payload")?;
            written = written + n - 1;
        }
    }

    #[cfg(not(windows))]
    {
        written = device.write(data).context("write payload")?;
    }

    anyhow::ensure!(
        written == data.len(),
        "incomplete write: {written} of {} bytes",
        data.len()
    );

    Ok(())
}

fn prepend_byte_and_offset(data: &[u8], offset: usize) -> [u8; 65] {
    let mut result: [u8; 65] = [0u8; 65];
    result[1] = 0x0;
    result[1..].copy_from_slice(&data[offset..offset + 64]);
    result
}
