//! Connect to an LED badge via Bluetooth Low Energy (BLE)

use std::time::Duration;

use anyhow::{Context, Result};
use async_std::task;
use btleplug::{
    api::{Central as _, Manager as _, Peripheral as _, ScanFilter, WriteType},
    platform::{Manager, Peripheral},
};

use crate::protocol::PayloadBuffer;

const BADGE_SERVICE_UUID_STR: &str = "0000fee0-0000-1000-8000-00805f9b34fb";
const BADGE_CHAR_UUID_STR: &str = "0000fee1-0000-1000-8000-00805f9b34fb";
const BADGE_BLE_DEVICE_NAME: &str = "LSLED";
const BLE_CHAR_CHUNK_SIZE: usize = 16;

/// A discovered BLE device
pub struct Device {
    peripheral: Peripheral,
}

impl Device {
    /// Return all supported devices that are found in two seconds.
    ///
    /// Returns all badges that are in BLE range and are in Bluetooth transfer mode.
    pub async fn enumerate() -> Result<Vec<Self>> {
        Self::enumerate_duration(Duration::from_secs(2)).await
    }

    /// Return all supported devices that are found in the given duration.
    ///
    /// Returns all badges that are in BLE range and are in Bluetooth transfer mode.
    /// # Panics
    /// This function panics if it is unable to access the Bluetooth adapter.
    pub async fn enumerate_duration(scan_duration: Duration) -> Result<Vec<Self>> {
        // Run device scan
        let manager = Manager::new().await?;
        let adapter = manager.adapters().await?.pop();
        if adapter.is_none() {
            return Err(anyhow::anyhow!("No Bluetooth adapter found"));
        }

        let adapter = adapter.unwrap();
        adapter.start_scan(ScanFilter::default()).await?;
        task::sleep(scan_duration).await;

        // Filter for badge devices
        let mut led_badges = vec![];
        for p in adapter.peripherals().await? {
            if Self::is_badge_device(&p).await {
                led_badges.push(p);
            }
        }

        led_badges
            .into_iter()
            .map(|p| Ok(Self { peripheral: p }))
            .collect()
    }

    async fn is_badge_device(peripheral: &Peripheral) -> bool {
        // Check whether the BLE device has the service UUID we're looking for
        // and also the correct name.
        // The service uuid is also by devices that are not LED badges, so
        // the name check is also necessary.
        let props = peripheral.properties().await;
        if props.is_err() {
            return false;
        }

        if let Some(props) = props.unwrap() {
            if props.local_name.is_none() {
                return false;
            }

            if props.local_name.as_ref().unwrap() != BADGE_BLE_DEVICE_NAME {
                return false;
            }

            props
                .services
                .iter()
                .any(|uuid| uuid.to_string() == BADGE_SERVICE_UUID_STR)
        } else {
            false
        }
    }

    /// Return the single supported device
    ///
    /// This function returns an error if no device could be found
    /// or if multiple devices would match.
    pub async fn single() -> Result<Self> {
        let mut devices = Self::enumerate().await?.into_iter();
        let device = devices.next().context("no device found")?;
        anyhow::ensure!(devices.next().is_none(), "multiple devices found");
        Ok(device)
    }

    /// Write a payload to the device.
    ///
    /// This function connects to the device, writes the payload and disconnects.
    /// When the device went out of range between discovering it
    /// and writing the payload, an error is returned.
    /// # Panics
    /// This functions panics if the BLE device does not have the expected badge characteristic.
    pub async fn write(&self, payload: PayloadBuffer) -> Result<()> {
        // Connect and discover services
        self.peripheral.connect().await?;
        if let Err(error) = self.peripheral.discover_services().await {
            self.peripheral.disconnect().await?;
            return Err(error.into());
        }

        // Get characteristics
        let characteristics = self.peripheral.characteristics();
        let badge_char = characteristics
            .iter()
            .find(|c| c.uuid.to_string() == BADGE_CHAR_UUID_STR);

        if badge_char.is_none() {
            return Err(anyhow::anyhow!("Badge characteristic not found"));
        }
        let badge_char = badge_char.unwrap();

        // Write payload
        let bytes = payload.into_padded_bytes();
        let data = bytes.as_ref();

        anyhow::ensure!(
            data.len() % BLE_CHAR_CHUNK_SIZE == 0,
            format!(
                "Payload size must be a multiple of {} bytes",
                BLE_CHAR_CHUNK_SIZE
            )
        );

        // the device will brick itself if the payload is too long (more then 8192 bytes)
        anyhow::ensure!(data.len() <= 8192, "payload too long (max 8192 bytes)");

        for chunk in data.chunks(BLE_CHAR_CHUNK_SIZE) {
            let write_result = self
                .peripheral
                .write(badge_char, chunk, WriteType::WithoutResponse)
                .await;

            if let Err(error) = write_result {
                self.peripheral.disconnect().await?;
                return Err(anyhow::anyhow!("Error writing payload chunk: {:?}", error));
            }
        }

        self.peripheral.disconnect().await?;
        Ok(())
    }
}
