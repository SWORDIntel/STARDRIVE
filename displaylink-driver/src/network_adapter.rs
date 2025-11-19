// DisplayLink Network Adapter Support (Interface 5)
// Phase 6: Network adapter functionality for DisplayLink USB docks
//
// This module provides basic network adapter support for DisplayLink devices
// that expose a network interface (MI_05 from Windows driver analysis)

use rusb::DeviceHandle;
use std::sync::{Arc, Mutex};

/// Network adapter interface number
pub const NETWORK_INTERFACE: u8 = 5;

/// Network adapter endpoints
pub const NET_BULK_OUT_ENDPOINT: u8 = 0x05;
pub const NET_BULK_IN_ENDPOINT: u8 = 0x85;

/// Network adapter manager
pub struct NetworkAdapter {
    usb_handle: Arc<Mutex<DeviceHandle<rusb::Context>>>,
    device_id: String,
    enabled: bool,
}

impl NetworkAdapter {
    pub fn new(usb_handle: Arc<Mutex<DeviceHandle<rusb::Context>>>, device_id: String) -> Self {
        NetworkAdapter {
            usb_handle,
            device_id,
            enabled: false,
        }
    }

    /// Initialize the network adapter interface
    pub fn initialize(&mut self) -> Result<(), String> {
        let handle = self.usb_handle.lock().unwrap();

        println!(
            "[{}] Initializing network adapter (interface {})",
            self.device_id, NETWORK_INTERFACE
        );

        // Check if kernel driver is active
        match handle.kernel_driver_active(NETWORK_INTERFACE) {
            Ok(true) => {
                println!(
                    "[{}] Kernel network driver is active (CDC NCM)",
                    self.device_id
                );
                // Don't detach - let kernel handle network interface
                return Ok(());
            }
            Ok(false) => {
                println!("[{}] No kernel network driver attached", self.device_id);
            }
            Err(e) => {
                println!(
                    "[{}] Cannot check network driver status: {}",
                    self.device_id, e
                );
            }
        }

        // Try to claim the network interface
        match handle.claim_interface(NETWORK_INTERFACE) {
            Ok(_) => {
                println!("[{}] ✓ Network interface claimed", self.device_id);
                self.enabled = true;
                Ok(())
            }
            Err(e) => {
                // Don't fail if we can't claim - display still works
                println!(
                    "[{}] Network interface not available: {}",
                    self.device_id, e
                );
                Ok(())
            }
        }
    }

    /// Get network adapter status
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get device ID
    pub fn device_id(&self) -> &str {
        &self.device_id
    }
}

impl Drop for NetworkAdapter {
    fn drop(&mut self) {
        if self.enabled {
            if let Ok(handle) = self.usb_handle.lock() {
                let _ = handle.release_interface(NETWORK_INTERFACE);
                println!("[{}] Network interface released", self.device_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_constants() {
        assert_eq!(NETWORK_INTERFACE, 5);
        assert_eq!(NET_BULK_OUT_ENDPOINT, 0x05);
        assert_eq!(NET_BULK_IN_ENDPOINT, 0x85);
    }

    #[test]
    fn test_network_adapter_creation() {
        // Note: This test requires a mock USB handle
        // In production, would use proper mocking framework
        println!("Network adapter module compiled successfully");
    }
}
