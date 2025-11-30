use crate::driver::DisplayLinkDriver;
use crate::evdi::{EVDI_INVALID_HANDLE, evdi_add_device, evdi_enable_cursor_events, evdi_open};
use rusb::{Device, UsbContext};
use std::collections::HashSet;
use std::env;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

// DisplayLink Vendor ID and Product ID (StarTech USB35DOCK)
pub const DISPLAYLINK_VID: u16 = 0x17e9;
pub const DISPLAYLINK_PID: u16 = 0x4307;

// Helper for verbose logging
pub static VERBOSE_LOG: OnceLock<bool> = OnceLock::new();

pub fn verbose_enabled() -> bool {
    *VERBOSE_LOG.get_or_init(|| env::var("DISPLAYLINK_DRIVER_VERBOSE").is_ok())
}

macro_rules! vprintln {
    ($($arg:tt)*) => {
        if crate::manager::verbose_enabled() {
            println!($($arg)*);
        }
    };
}



pub struct DisplayLinkManager {
    drivers: Arc<Mutex<HashSet<String>>>,
    context: Arc<rusb::Context>,
}

impl DisplayLinkManager {
    pub fn new(context: rusb::Context) -> Self {
        DisplayLinkManager {
            drivers: Arc::new(Mutex::new(HashSet::new())),
            context: Arc::new(context),
        }
    }

    fn initialize_device(&self, device: Device<rusb::Context>) -> Result<(), String> {
        let device_desc = device
            .device_descriptor()
            .map_err(|e| format!("Failed to get device descriptor: {}", e))?;

        if device_desc.vendor_id() != DISPLAYLINK_VID || device_desc.product_id() != DISPLAYLINK_PID
        {
            return Err("Not a DisplayLink device".to_string());
        }

        let device_id = format!("{}:{}", device.bus_number(), device.address());

        // Check if already initialized
        {
            let drivers = self.drivers.lock().unwrap();
            if drivers.contains(&device_id) {
                return Ok(())
            }
        }

        println!("Initializing DisplayLink device: {}", device_id);
        vprintln!(
            "  Device descriptor: bus {} addr {} (VID:PID {:04X}:{:04X})",
            device.bus_number(),
            device.address(),
            device_desc.vendor_id(),
            device_desc.product_id()
        );
        println!(
            "  Bus: {}, Address: {}",
            device.bus_number(),
            device.address()
        );
        println!(
            "  VID: 0x{:04X}, PID: 0x{:04X}",
            device_desc.vendor_id(),
            device_desc.product_id()
        );

        let handle = device
            .open()
            .map_err(|e| format!("Failed to open device: {}", e))?;

        // Create EVDI device
        let evdi_handle = unsafe {
            let card_no = evdi_add_device();
            if card_no < 0 {
                return Err("Failed to add EVDI device".to_string());
            }
            println!("  Created EVDI device: /dev/dri/card{}", card_no);

            let handle = evdi_open(card_no);
            if handle == EVDI_INVALID_HANDLE {
                return Err("Failed to open EVDI device".to_string());
            }

            // Disable hardware cursor events so the OS renders the cursor into the framebuffer.
            // Since we don't implement the cursor handlers, enabling this would make the cursor invisible.
            evdi_enable_cursor_events(handle, false);
            handle
        };

        // Create driver instance
        let mut driver = DisplayLinkDriver::new(device_id.clone(), evdi_handle, handle);

        // Initialize USB device
        driver.initialize_device()?;

        println!("  ✓ Device initialized successfully");

        // Spawn event loop thread
        let device_id_clone = device_id.clone();
        thread::spawn(move || {
            if let Err(e) = driver.run() {
                eprintln!("[{}] Driver error: {}", device_id_clone, e);
            }
        });

        // Mark device as active
        {
            let mut drivers = self.drivers.lock().unwrap();
            drivers.insert(device_id);
        }

        Ok(())
    }

    fn scan_devices(&self) -> Result<(), String> {
        let devices = self
            .context
            .devices()
            .map_err(|e| format!("Failed to list devices: {}", e))?;

        for device in devices.iter() {
            if let Ok(desc) = device.device_descriptor() {
                if desc.vendor_id() == DISPLAYLINK_VID && desc.product_id() == DISPLAYLINK_PID {
                    if let Err(e) = self.initialize_device(device) {
                        eprintln!("Failed to initialize device: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn run(&self) -> Result<(), String> {
        println!("DisplayLink Manager running with hot-plug support");
        vprintln!("  Starting hot-plug scan loop");
        println!(
            "Monitoring for DisplayLink devices (VID: 0x{:04X}, PID: 0x{:04X})",
            DISPLAYLINK_VID,
            DISPLAYLINK_PID
        );
        println!("Press Ctrl+C to exit\n");

        // Initial scan
        self.scan_devices()?;

        // Monitor for new devices periodically (reduced frequency to lower system load)
        loop {
            thread::sleep(Duration::from_secs(1)); // Fast polling (1s) for responsive hot-plug
            vprintln!("  Sleeping before next hot-plug poll");
            self.scan_devices()?;
        }
    }
}