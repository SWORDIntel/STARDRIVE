// Allow warnings for auto-generated bindings
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

mod displaylink_protocol;
mod network_adapter;

use rusb::{Device, DeviceDescriptor, DeviceHandle, UsbContext};
use std::collections::HashSet;
use std::env;
use std::ffi::c_void;
use std::ptr;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use displaylink_protocol::*;
use network_adapter::NetworkAdapter;

// Include auto-generated EVDI bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// Define EVDI_INVALID_HANDLE (bindgen doesn't handle C macros)
const EVDI_INVALID_HANDLE: evdi_handle = ptr::null_mut();

// DisplayLink Vendor ID and Product ID (StarTech USB35DOCK)
const DISPLAYLINK_VID: u16 = 0x17e9;
const DISPLAYLINK_PID: u16 = 0x4307;

// USB interface and endpoint configuration
const DISPLAY_INTERFACE: u8 = 0; // MI_00 from Windows driver analysis
const NETWORK_INTERFACE: u8 = 5; // MI_05 from Windows driver analysis
const BULK_OUT_ENDPOINT: u8 = 0x01;
const BULK_IN_ENDPOINT: u8 = 0x81;

// Default EDID for a 1920x1080 display (256 bytes with CEA-861 extension)
const DEFAULT_EDID: &[u8] = &[
    // Block 0: Base EDID (128 bytes)
    0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x10, 0xAC, 0x4F, 0xA0, 0x4C, 0x50, 0x39, 0x30,
    0x1E, 0x1D, 0x01, 0x04, 0xA5, 0x34, 0x20, 0x78, 0xFB, 0x6C, 0xE5, 0xA5, 0x55, 0x50, 0xA0, 0x23,
    0x0B, 0x50, 0x54, 0xA5, 0x4B, 0x00, 0x81, 0x80, 0xA9, 0x40, 0xD1, 0x00, 0x71, 0x4F, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x02, 0x3A, 0x80, 0x18, 0x71, 0x38, 0x2D, 0x40, 0x58, 0x2C,
    0x45, 0x00, 0x09, 0x25, 0x21, 0x00, 0x00, 0x1E, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x48, 0x56, 0x4E,
    0x44, 0x59, 0x30, 0x39, 0x50, 0x4C, 0x00, 0x0A, 0x20, 0x20, 0x00, 0x00, 0x00, 0xFC, 0x00, 0x44,
    0x45, 0x4C, 0x4C, 0x20, 0x50, 0x32, 0x34, 0x31, 0x34, 0x48, 0x0A, 0x20, 0x00, 0x00, 0x00, 0xFD,
    0x00, 0x38, 0x4C, 0x1E, 0x53, 0x11, 0x00, 0x0A, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x01, 0x9B,
    // Block 1: CEA-861 Extension Block (128 bytes)
    0x02, 0x03, 0x1D, 0xF1, 0x4B, 0x90, 0x05, 0x04, 0x03, 0x02, 0x07, 0x16, 0x01, 0x06, 0x11, 0x12,
    0x15, 0x13, 0x14, 0x1F, 0x20, 0x23, 0x09, 0x07, 0x07, 0x83, 0x01, 0x00, 0x00, 0x65, 0x03, 0x0C,
    0x00, 0x10, 0x00, 0x02, 0x3A, 0x80, 0x18, 0x71, 0x38, 0x2D, 0x40, 0x58, 0x2C, 0x45, 0x00, 0x09,
    0x25, 0x21, 0x00, 0x00, 0x1E, 0x01, 0x1D, 0x80, 0x18, 0x71, 0x1C, 0x16, 0x20, 0x58, 0x2C, 0x25,
    0x00, 0x09, 0x25, 0x21, 0x00, 0x00, 0x9E, 0x01, 0x1D, 0x00, 0x72, 0x51, 0xD0, 0x1E, 0x20, 0x6E,
    0x28, 0x55, 0x00, 0x09, 0x25, 0x21, 0x00, 0x00, 0x1E, 0x8C, 0x0A, 0xD0, 0x8A, 0x20, 0xE0, 0x2D,
    0x10, 0x10, 0x3E, 0x96, 0x00, 0x09, 0x25, 0x21, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x12,
];

// Wrapper to make evdi_handle Send (EVDI is thread-safe in practice)
struct SendEvdiHandle(evdi_handle);
unsafe impl Send for SendEvdiHandle {}
unsafe impl Sync for SendEvdiHandle {}

// Multi-monitor manager with hot-plug support
static VERBOSE_LOG: OnceLock<bool> = OnceLock::new();

fn verbose_enabled() -> bool {
    *VERBOSE_LOG.get_or_init(|| env::var("DISPLAYLINK_DRIVER_VERBOSE").is_ok())
}

macro_rules! vprintln {
    ($($arg:tt)*) => {
        if verbose_enabled() {
            println!($($arg)*);
        }
    };
}

struct DisplayLinkManager {
    drivers: Arc<Mutex<HashSet<String>>>,
    context: Arc<rusb::Context>,
}

// Driver state
struct DisplayLinkDriver {
    device_id: String,
    evdi_handle: SendEvdiHandle,
    usb_handle: Arc<Mutex<DeviceHandle<rusb::Context>>>,
    current_mode: Option<evdi_mode>,
    buffers: Vec<FrameBuffer>,
    compressor: RLECompressor,
    cmd_builder: CommandBuilder,
    running: Arc<Mutex<bool>>,
    network_adapter: Option<NetworkAdapter>,
}

struct FrameBuffer {
    id: i32,
    data: Vec<u8>,
    width: i32,
    height: i32,
    stride: i32,
}

impl DisplayLinkDriver {
    fn new(
        device_id: String,
        evdi_handle: evdi_handle,
        usb_handle: DeviceHandle<rusb::Context>,
    ) -> Self {
        let usb_handle_arc = Arc::new(Mutex::new(usb_handle));

        // Initialize network adapter
        let network_adapter = NetworkAdapter::new(usb_handle_arc.clone(), device_id.clone());

        DisplayLinkDriver {
            device_id,
            evdi_handle: SendEvdiHandle(evdi_handle),
            usb_handle: usb_handle_arc,
            current_mode: None,
            buffers: Vec::new(),
            compressor: RLECompressor::new(),
            cmd_builder: CommandBuilder::new(),
            running: Arc::new(Mutex::new(true)),
            network_adapter: Some(network_adapter),
        }
    }

    // Initialize the DisplayLink device via USB
    fn initialize_device(&mut self) -> Result<(), String> {
        {
            let handle = self.usb_handle.lock().unwrap();

            // Detach kernel driver if active (Linux only)
            match handle.kernel_driver_active(DISPLAY_INTERFACE) {
                Ok(true) => {
                    println!(
                        "Detaching kernel driver from interface {}",
                        DISPLAY_INTERFACE
                    );
                    handle
                        .detach_kernel_driver(DISPLAY_INTERFACE)
                        .map_err(|e| format!("Failed to detach kernel driver: {}", e))?;
                }
                Ok(false) => println!("No kernel driver attached"),
                Err(e) => println!("Cannot check kernel driver status: {}", e),
            }

            // Claim the display interface
            println!("Claiming interface {}", DISPLAY_INTERFACE);
            handle
                .claim_interface(DISPLAY_INTERFACE)
                .map_err(|e| format!("Failed to claim interface: {}", e))?;
        } // Drop handle lock here

        // Initialize network adapter (non-fatal if fails)
        if let Some(ref mut net_adapter) = self.network_adapter {
            let _ = net_adapter.initialize();
        }

        // Send initialization sequence to DisplayLink device
        self.send_init_sequence()?;

        Ok(())
    }

    // Send initialization commands to DisplayLink device
    fn send_init_sequence(&mut self) -> Result<(), String> {
        println!("Initializing DisplayLink device...");
        vprintln!("  DL-3000 series: testing bulk endpoint");

        // For DL-3000, try sending a minimal test packet first
        // This will tell us if bulk transfers work at all
        let test_data = vec![0x00; 64]; // Simple 64-byte zero packet
        vprintln!("  Sending test packet ({} bytes of zeros)", test_data.len());

        match self.send_bulk_data(&test_data) {
            Ok(_) => {
                println!("  ✓ Bulk endpoint accepts data!");
                // Now try a register write command
                let blank_cmd = self.cmd_builder.blank_screen(true).to_vec();
                vprintln!(
                    "  Trying register write command ({} bytes)",
                    blank_cmd.len()
                );
                self.send_bulk_data(&blank_cmd)?;
                println!("  ✓ Register write succeeded");
            }
            Err(e) => {
                println!("  ✗ Bulk endpoint rejected test data: {}", e);
                return Err(format!("Bulk endpoint test failed: {}", e));
            }
        }

        Ok(())
    }

    // Send framebuffer data to DisplayLink device
    fn send_framebuffer(&mut self, buffer: &FrameBuffer) -> Result<(), String> {
        println!(
            "Compressing framebuffer: {}x{}",
            buffer.width, buffer.height
        );

        // Compress framebuffer using RLE
        let compressed = self
            .compressor
            .compress(&buffer.data, buffer.width as usize, buffer.height as usize)
            .to_vec();

        println!(
            "  Compressed {} bytes -> {} bytes",
            buffer.data.len(),
            compressed.len()
        );

        // Set damage rectangle (full screen update)
        let damage_cmd = self
            .cmd_builder
            .damage_rect(0, 0, buffer.width as u16, buffer.height as u16)
            .to_vec();
        self.send_bulk_data(&damage_cmd)?;

        // Send compressed framebuffer data in chunks
        self.send_bulk_data(&compressed)?;

        // Sync/flush command
        let sync_cmd = self.cmd_builder.sync().to_vec();
        self.send_bulk_data(&sync_cmd)?;

        println!("  ✓ Framebuffer sent");

        Ok(())
    }

    // Send mode set command to DisplayLink device
    fn send_mode_set(&mut self, mode: &DisplayMode) -> Result<(), String> {
        println!(
            "Setting display mode: {}x{}@{}Hz",
            mode.width, mode.height, mode.refresh_rate
        );

        let mode_cmd = self.cmd_builder.set_mode(mode).to_vec();
        self.send_bulk_data(&mode_cmd)?;

        // Unblank the screen after mode set
        let unblank_cmd = self.cmd_builder.blank_screen(false).to_vec();
        self.send_bulk_data(&unblank_cmd)?;

        println!("  ✓ Mode set complete");

        Ok(())
    }

    // Send data via USB bulk transfer
    fn send_bulk_data(&self, data: &[u8]) -> Result<(), String> {
        let handle = self.usb_handle.lock().unwrap();

        // Split into chunks if necessary
        for chunk in data.chunks(DL_MAX_TRANSFER_SIZE) {
            handle
                .write_bulk(BULK_OUT_ENDPOINT, chunk, BULK_TIMEOUT)
                .map_err(|e| format!("Bulk transfer failed: {}", e))?;
        }

        Ok(())
    }

    // Register a framebuffer with EVDI
    fn register_buffer(&mut self, width: i32, height: i32) -> Result<i32, String> {
        let buffer_id = self.buffers.len() as i32;
        let stride = width * 4; // 4 bytes per pixel (BGRA)
        let buffer_size = (stride * height) as usize;

        let mut framebuffer = FrameBuffer {
            id: buffer_id,
            data: vec![0u8; buffer_size],
            width,
            height,
            stride,
        };

        let evdi_buf = evdi_buffer {
            id: buffer_id,
            buffer: framebuffer.data.as_mut_ptr() as *mut c_void,
            width,
            height,
            stride,
            rects: ptr::null_mut(),
            rect_count: 0,
        };

        unsafe {
            evdi_register_buffer(self.evdi_handle.0, evdi_buf);
        }

        self.buffers.push(framebuffer);
        println!("Registered buffer {} ({}x{})", buffer_id, width, height);

        Ok(buffer_id)
    }

    // Handle EVDI events
    fn handle_events(&mut self) {
        unsafe extern "C" fn dpms_handler(dpms_mode: i32, user_data: *mut c_void) {
            let driver = &mut *(user_data as *mut DisplayLinkDriver);
            println!(
                "[{}] DPMS mode changed: {} ({})",
                driver.device_id,
                dpms_mode,
                match dpms_mode {
                    0 => "ON",
                    1 => "STANDBY",
                    2 => "SUSPEND",
                    3 => "OFF",
                    _ => "UNKNOWN",
                }
            );

            // Blank or unblank screen based on DPMS mode
            let should_blank = dpms_mode != 0; // Blank for all modes except ON
            let blank_cmd = driver.cmd_builder.blank_screen(should_blank).to_vec();

            if let Err(e) = driver.send_bulk_data(&blank_cmd) {
                eprintln!("[{}] Failed to set DPMS mode: {}", driver.device_id, e);
            }
        }

        unsafe extern "C" fn mode_changed_handler(mode: evdi_mode, user_data: *mut c_void) {
            let driver = &mut *(user_data as *mut DisplayLinkDriver);
            println!(
                "[{}] Mode changed: {}x{}@{}Hz (dynamic resolution)",
                driver.device_id, mode.width, mode.height, mode.refresh_rate
            );
            driver.current_mode = Some(mode);

            // Calculate timing parameters based on resolution
            let (pixel_clock, hsync_start, hsync_end, htotal, vsync_start, vsync_end, vtotal) =
                match (mode.width, mode.height) {
                    (1920, 1080) => (
                        148500,
                        1920 + 88,
                        1920 + 88 + 44,
                        2200,
                        1080 + 4,
                        1080 + 4 + 5,
                        1125,
                    ),
                    (1280, 720) => (
                        74250,
                        1280 + 110,
                        1280 + 110 + 40,
                        1650,
                        720 + 5,
                        720 + 5 + 5,
                        750,
                    ),
                    (1024, 768) => (
                        65000,
                        1024 + 24,
                        1024 + 24 + 136,
                        1344,
                        768 + 3,
                        768 + 3 + 6,
                        806,
                    ),
                    _ => {
                        // Generic timing for other resolutions
                        let h_blank = (mode.width / 5) as u32;
                        let v_blank = (mode.height / 30) as u32;
                        let pixel_clock = (mode.width as u32 + h_blank)
                            * (mode.height as u32 + v_blank)
                            * mode.refresh_rate as u32
                            / 1000;
                        (
                            pixel_clock,
                            mode.width as u32 + h_blank / 2,
                            mode.width as u32 + h_blank / 2 + h_blank / 10,
                            mode.width as u32 + h_blank,
                            mode.height as u32 + v_blank / 2,
                            mode.height as u32 + v_blank / 2 + v_blank / 10,
                            mode.height as u32 + v_blank,
                        )
                    }
                };

            // Create DisplayLink mode configuration
            let dl_mode = DisplayMode {
                width: mode.width as u32,
                height: mode.height as u32,
                refresh_rate: mode.refresh_rate as u32,
                pixel_clock,
                hsync_start,
                hsync_end,
                htotal,
                vsync_start,
                vsync_end,
                vtotal,
            };

            // Send mode to DisplayLink device
            if let Err(e) = driver.send_mode_set(&dl_mode) {
                eprintln!(
                    "[{}] Failed to set DisplayLink mode: {}",
                    driver.device_id, e
                );
                return;
            }

            // Register new buffer for new mode
            if let Err(e) = driver.register_buffer(mode.width, mode.height) {
                eprintln!("[{}] Failed to register buffer: {}", driver.device_id, e);
            }
        }

        unsafe extern "C" fn update_ready_handler(buffer_id: i32, user_data: *mut c_void) {
            let driver = &mut *(user_data as *mut DisplayLinkDriver);
            println!("Update ready for buffer {}", buffer_id);

            // Request pixel data from EVDI
            evdi_grab_pixels(driver.evdi_handle.0, ptr::null_mut(), ptr::null_mut());

            // Send framebuffer to DisplayLink device
            // Find buffer and clone necessary data to avoid borrow issues
            if let Some(buffer_index) = driver.buffers.iter().position(|b| b.id == buffer_id) {
                // Create a temporary buffer reference
                let buffer = &driver.buffers[buffer_index];
                let buffer_copy = FrameBuffer {
                    id: buffer.id,
                    data: buffer.data.clone(),
                    width: buffer.width,
                    height: buffer.height,
                    stride: buffer.stride,
                };
                if let Err(e) = driver.send_framebuffer(&buffer_copy) {
                    eprintln!("Failed to send framebuffer: {}", e);
                }
            }
        }

        unsafe extern "C" fn crtc_state_handler(state: i32, _user_data: *mut c_void) {
            println!("CRTC state changed: {}", state);
        }

        unsafe extern "C" fn cursor_set_handler(cursor: evdi_cursor_set, _user_data: *mut c_void) {
            println!(
                "Cursor set: {}x{} at ({}, {})",
                cursor.width, cursor.height, cursor.hot_x, cursor.hot_y
            );
            // Handle cursor updates
        }

        unsafe extern "C" fn cursor_move_handler(
            cursor: evdi_cursor_move,
            _user_data: *mut c_void,
        ) {
            println!("Cursor moved to ({}, {})", cursor.x, cursor.y);
        }

        unsafe extern "C" fn ddcci_handler(_ddcci: evdi_ddcci_data, _user_data: *mut c_void) {
            println!("DDC/CI data received");
        }

        let mut event_context = evdi_event_context {
            dpms_handler: Some(dpms_handler),
            mode_changed_handler: Some(mode_changed_handler),
            update_ready_handler: Some(update_ready_handler),
            crtc_state_handler: Some(crtc_state_handler),
            cursor_set_handler: Some(cursor_set_handler),
            cursor_move_handler: Some(cursor_move_handler),
            ddcci_data_handler: Some(ddcci_handler),
            user_data: self as *mut _ as *mut c_void,
        };

        unsafe {
            evdi_handle_events(self.evdi_handle.0, &mut event_context);
        }
    }

    // Main event loop
    fn run(&mut self) -> Result<(), String> {
        println!(
            "[{}] Starting DisplayLink driver event loop...",
            self.device_id
        );

        loop {
            // Check if we should continue running
            {
                let running = self.running.lock().unwrap();
                if !*running {
                    println!("[{}] Stopping driver", self.device_id);
                    break;
                }
            }

            unsafe {
                let event_fd = evdi_get_event_ready(self.evdi_handle.0);
                if event_fd != -1 {
                    self.handle_events();
                }
            }

            // Small delay to prevent busy waiting
            thread::sleep(Duration::from_millis(10));
        }

        Ok(())
    }

    fn stop(&mut self) {
        let mut running = self.running.lock().unwrap();
        *running = false;
    }
}

impl Drop for DisplayLinkDriver {
    fn drop(&mut self) {
        println!("[{}] Shutting down DisplayLink driver...", self.device_id);

        // Disconnect from EVDI
        unsafe {
            evdi_disconnect(self.evdi_handle.0);
            evdi_close(self.evdi_handle.0);
        }

        // Release USB interface
        if let Ok(handle) = self.usb_handle.lock() {
            let _ = handle.release_interface(DISPLAY_INTERFACE);
        }
    }
}

impl DisplayLinkManager {
    fn new(context: rusb::Context) -> Self {
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
                return Ok(());
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

            evdi_connect(handle, DEFAULT_EDID.as_ptr(), DEFAULT_EDID.len() as u32, 0);

            evdi_enable_cursor_events(handle, true);
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

    fn run(&self) -> Result<(), String> {
        println!("DisplayLink Manager running with hot-plug support");
        vprintln!("  Starting hot-plug scan loop");
        println!(
            "Monitoring for DisplayLink devices (VID: 0x{:04X}, PID: 0x{:04X})",
            DISPLAYLINK_VID, DISPLAYLINK_PID
        );
        println!("Press Ctrl+C to exit\n");

        // Initial scan
        self.scan_devices()?;

        // Monitor for new devices periodically
        loop {
            thread::sleep(Duration::from_secs(2));
            vprintln!("  Sleeping before next hot-plug poll");
            self.scan_devices()?;
        }
    }

    fn device_count(&self) -> usize {
        self.drivers.lock().unwrap().len()
    }
}

fn main() {
    println!("DisplayLink Rust Driver v0.2.0 - Phase 6");
    println!("=========================================");
    println!("Features: Multi-monitor, Hot-plug, Power management");
    println!();

    // Initialize EVDI library
    unsafe {
        let mut version = evdi_lib_version {
            version_major: 0,
            version_minor: 0,
            version_patchlevel: 0,
        };
        evdi_get_lib_version(&mut version);
        println!(
            "EVDI library version: {}.{}.{}",
            version.version_major, version.version_minor, version.version_patchlevel
        );
    }

    // Initialize USB context and manager
    match rusb::Context::new() {
        Ok(context) => {
            println!("USB context initialized.\n");

            // Create DisplayLink manager
            let manager = DisplayLinkManager::new(context);

            // Run manager with hot-plug support
            if let Err(e) = manager.run() {
                eprintln!("Manager error: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Could not initialize USB context: {}", e);
        }
    }
}

fn find_displaylink_device<T: UsbContext>(
    context: &mut T,
) -> Option<(Device<T>, DeviceDescriptor)> {
    match context.devices() {
        Ok(devices) => {
            for device in devices.iter() {
                if let Ok(device_desc) = device.device_descriptor() {
                    if device_desc.vendor_id() == DISPLAYLINK_VID
                        && device_desc.product_id() == DISPLAYLINK_PID
                    {
                        return Some((device, device_desc));
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error listing devices: {}", e);
        }
    }
    None
}
