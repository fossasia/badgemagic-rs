//! Connect to an LED badge via Bluetooth Low Energy (BLE)

use std::time::Duration;

use anyhow::{Context, Result};
use btleplug::{
    api::{bleuuid, Central as _, Manager as _, Peripheral as _, ScanFilter, WriteType},
    platform::{Manager, Peripheral},
};
use tokio::time;
use uuid::Uuid;

use crate::protocol::PayloadBuffer;

/// `0000fee0-0000-1000-8000-00805f9b34fb`
const BADGE_SERVICE_UUID: Uuid = bleuuid::uuid_from_u16(0xfee0);
/// `0000fee1-0000-1000-8000-00805f9b34fb`
const BADGE_CHAR_UUID: Uuid = bleuuid::uuid_from_u16(0xfee1);

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
        let manager = Manager::new().await.context("create BLE manager")?;
        let adapters = manager
            .adapters()
            .await
            .context("enumerate bluetooth adapters")?;
        let adapter = adapters.first().context("no bluetooth adapter found")?;

        adapter
            .start_scan(ScanFilter {
                services: vec![BADGE_SERVICE_UUID],
            })
            .await
            .context("bluetooth scan start")?;
        time::sleep(scan_duration).await;

        // Filter for badge devices
        let mut led_badges = vec![];
        for p in adapter
            .peripherals()
            .await
            .context("enumerating bluetooth devices")?
        {
            if let Some(badge) = Self::from_peripheral(p).await {
                led_badges.push(badge);
            }
        }

        Ok(led_badges)
    }

    async fn from_peripheral(peripheral: Peripheral) -> Option<Self> {
        // Check whether the BLE device has the service UUID we're looking for
        // and also the correct name.
        // The service uuid is also by devices that are not LED badges, so
        // the name check is also necessary.
        let props = peripheral.properties().await.ok()??;

        let local_name = props.local_name.as_ref()?;
        if local_name != BADGE_BLE_DEVICE_NAME {
            return None;
        }

        if props
            .services
            .iter()
            .any(|uuid| *uuid == BADGE_SERVICE_UUID)
        {
            Some(Self { peripheral })
        } else {
            None
        }
    }

    /// Return the single supported device
    ///
    /// This function returns an error if no device could be found
    /// or if multiple devices would match.
    pub async fn single() -> Result<Self> {
        let mut devices = Self::enumerate()
            .await
            .context("enumerating badges")?
            .into_iter();
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
        self.peripheral
            .connect()
            .await
            .context("bluetooth device connect")?;

        let result = self.write_connected(payload).await;
        let disconnect_result = self.peripheral.disconnect().await;

        if result.is_ok() {
            // Write succesful, return disconnect result
            Ok(disconnect_result?)
        } else {
            // Write failed, return write result and ignore disconnect result
            result
        }
    }

    async fn write_connected(&self, payload: PayloadBuffer) -> Result<()> {
        // Get characteristic
        self.peripheral
            .discover_services()
            .await
            .context("discovering services")?;
        let characteristics = self.peripheral.characteristics();
        let badge_char = characteristics
            .iter()
            .find(|c| c.uuid == BADGE_CHAR_UUID)
            .context("badge characteristic not found")?;

        // Write payload
        let bytes = payload.into_padded_bytes();
        let data = bytes.as_ref();

        anyhow::ensure!(
            data.len() % BLE_CHAR_CHUNK_SIZE == 0,
            "Payload size must be a multiple of {} bytes",
            BLE_CHAR_CHUNK_SIZE
        );

        // the device will brick itself if the payload is too long (more then 8192 bytes)
        anyhow::ensure!(data.len() <= 8192, "payload too long (max 8192 bytes)");

        for chunk in data.chunks(BLE_CHAR_CHUNK_SIZE) {
            self.peripheral
                .write(badge_char, chunk, WriteType::WithoutResponse)
                .await
                .context("writing payload chunk")?;
        }

        Ok(())
    }
}
