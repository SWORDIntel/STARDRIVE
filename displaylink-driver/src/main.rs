// Allow warnings for auto-generated bindings
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

mod displaylink_protocol;

use rusb::{Device, DeviceDescriptor, DeviceHandle, UsbContext};
use std::ptr;
use std::ffi::c_void;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::thread;

use displaylink_protocol::*;

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

// Default EDID for a 1920x1080 display
const DEFAULT_EDID: &[u8] = &[
    0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x10, 0xAC, 0x4F, 0xA0,
    0x4C, 0x50, 0x39, 0x30, 0x1E, 0x1D, 0x01, 0x04, 0xA5, 0x34, 0x20, 0x78,
    0xFB, 0x6C, 0xE5, 0xA5, 0x55, 0x50, 0xA0, 0x23, 0x0B, 0x50, 0x54, 0xA5,
    0x4B, 0x00, 0x81, 0x80, 0xA9, 0x40, 0xD1, 0x00, 0x71, 0x4F, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x02, 0x3A, 0x80, 0x18, 0x71, 0x38,
    0x2D, 0x40, 0x58, 0x2C, 0x45, 0x00, 0x09, 0x25, 0x21, 0x00, 0x00, 0x1E,
    0x00, 0x00, 0x00, 0xFF, 0x00, 0x48, 0x56, 0x4E, 0x44, 0x59, 0x30, 0x39,
    0x50, 0x4C, 0x00, 0x0A, 0x20, 0x20, 0x00, 0x00, 0x00, 0xFC, 0x00, 0x44,
    0x45, 0x4C, 0x4C, 0x20, 0x50, 0x32, 0x34, 0x31, 0x34, 0x48, 0x0A, 0x20,
    0x00, 0x00, 0x00, 0xFD, 0x00, 0x38, 0x4C, 0x1E, 0x53, 0x11, 0x00, 0x0A,
    0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x01, 0x08,
];

// Driver state
struct DisplayLinkDriver {
    evdi_handle: evdi_handle,
    usb_handle: Arc<Mutex<DeviceHandle<rusb::Context>>>,
    current_mode: Option<evdi_mode>,
    buffers: Vec<FrameBuffer>,
    compressor: RLECompressor,
    cmd_builder: CommandBuilder,
}

struct FrameBuffer {
    id: i32,
    data: Vec<u8>,
    width: i32,
    height: i32,
    stride: i32,
}

impl DisplayLinkDriver {
    fn new(evdi_handle: evdi_handle, usb_handle: DeviceHandle<rusb::Context>) -> Self {
        DisplayLinkDriver {
            evdi_handle,
            usb_handle: Arc::new(Mutex::new(usb_handle)),
            current_mode: None,
            buffers: Vec::new(),
            compressor: RLECompressor::new(),
            cmd_builder: CommandBuilder::new(),
        }
    }

    // Initialize the DisplayLink device via USB
    fn initialize_device(&mut self) -> Result<(), String> {
        {
            let handle = self.usb_handle.lock().unwrap();

            // Detach kernel driver if active (Linux only)
            match handle.kernel_driver_active(DISPLAY_INTERFACE) {
                Ok(true) => {
                    println!("Detaching kernel driver from interface {}", DISPLAY_INTERFACE);
                    handle.detach_kernel_driver(DISPLAY_INTERFACE)
                        .map_err(|e| format!("Failed to detach kernel driver: {}", e))?;
                }
                Ok(false) => println!("No kernel driver attached"),
                Err(e) => println!("Cannot check kernel driver status: {}", e),
            }

            // Claim the display interface
            println!("Claiming interface {}", DISPLAY_INTERFACE);
            handle.claim_interface(DISPLAY_INTERFACE)
                .map_err(|e| format!("Failed to claim interface: {}", e))?;
        }  // Drop handle lock here

        // Send initialization sequence to DisplayLink device
        self.send_init_sequence()?;

        Ok(())
    }

    // Send initialization commands to DisplayLink device
    fn send_init_sequence(&mut self) -> Result<(), String> {
        let handle = self.usb_handle.lock().unwrap();

        println!("Initializing DisplayLink device...");

        // Initialize DisplayLink channel
        let request_type = USB_DIR_OUT | USB_TYPE_VENDOR | USB_RECIP_DEVICE;
        handle.write_control(
            request_type,
            DL_USB_REQUEST_CHANNEL,
            DL_CHAN_CMD_INIT,
            0,
            &[],
            CONTROL_TIMEOUT,
        ).map_err(|e| format!("Channel init failed: {}", e))?;

        println!("  ✓ Channel initialized");

        // Blank the screen initially
        let blank_cmd = self.cmd_builder.blank_screen(true).to_vec();
        drop(handle);  // Release lock before send_bulk_data
        self.send_bulk_data(&blank_cmd)?;

        println!("  ✓ Screen blanked");

        Ok(())
    }

    // Send framebuffer data to DisplayLink device
    fn send_framebuffer(&mut self, buffer: &FrameBuffer) -> Result<(), String> {
        println!("Compressing framebuffer: {}x{}", buffer.width, buffer.height);

        // Compress framebuffer using RLE
        let compressed = self.compressor.compress(
            &buffer.data,
            buffer.width as usize,
            buffer.height as usize,
        ).to_vec();

        println!("  Compressed {} bytes -> {} bytes",
            buffer.data.len(), compressed.len());

        // Set damage rectangle (full screen update)
        let damage_cmd = self.cmd_builder.damage_rect(
            0, 0,
            buffer.width as u16,
            buffer.height as u16,
        ).to_vec();
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
        println!("Setting display mode: {}x{}@{}Hz",
            mode.width, mode.height, mode.refresh_rate);

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
            handle.write_bulk(
                BULK_OUT_ENDPOINT,
                chunk,
                BULK_TIMEOUT,
            ).map_err(|e| format!("Bulk transfer failed: {}", e))?;
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
            evdi_register_buffer(self.evdi_handle, evdi_buf);
        }

        self.buffers.push(framebuffer);
        println!("Registered buffer {} ({}x{})", buffer_id, width, height);

        Ok(buffer_id)
    }

    // Handle EVDI events
    fn handle_events(&mut self) {
        unsafe extern "C" fn dpms_handler(dpms_mode: i32, _user_data: *mut c_void) {
            println!("DPMS mode changed: {}", dpms_mode);
            // Handle display power management
        }

        unsafe extern "C" fn mode_changed_handler(mode: evdi_mode, user_data: *mut c_void) {
            let driver = &mut *(user_data as *mut DisplayLinkDriver);
            println!("Mode changed: {}x{}@{}Hz",
                mode.width, mode.height, mode.refresh_rate);
            driver.current_mode = Some(mode);

            // Create DisplayLink mode configuration
            let dl_mode = DisplayMode {
                width: mode.width as u32,
                height: mode.height as u32,
                refresh_rate: mode.refresh_rate as u32,
                pixel_clock: 148500,  // Will be calculated based on mode
                hsync_start: mode.width as u32 + 88,
                hsync_end: mode.width as u32 + 88 + 44,
                htotal: mode.width as u32 + 280,
                vsync_start: mode.height as u32 + 4,
                vsync_end: mode.height as u32 + 4 + 5,
                vtotal: mode.height as u32 + 45,
            };

            // Send mode to DisplayLink device
            if let Err(e) = driver.send_mode_set(&dl_mode) {
                eprintln!("Failed to set DisplayLink mode: {}", e);
            }

            // Register new buffer for new mode
            if let Err(e) = driver.register_buffer(mode.width, mode.height) {
                eprintln!("Failed to register buffer: {}", e);
            }
        }

        unsafe extern "C" fn update_ready_handler(buffer_id: i32, user_data: *mut c_void) {
            let driver = &mut *(user_data as *mut DisplayLinkDriver);
            println!("Update ready for buffer {}", buffer_id);

            // Request pixel data from EVDI
            evdi_grab_pixels(driver.evdi_handle, ptr::null_mut(), ptr::null_mut());

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
            println!("Cursor set: {}x{} at ({}, {})",
                cursor.width, cursor.height, cursor.hot_x, cursor.hot_y);
            // Handle cursor updates
        }

        unsafe extern "C" fn cursor_move_handler(cursor: evdi_cursor_move, _user_data: *mut c_void) {
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
            evdi_handle_events(self.evdi_handle, &mut event_context);
        }
    }

    // Main event loop
    fn run(&mut self) -> Result<(), String> {
        println!("Starting DisplayLink driver event loop...");

        loop {
            unsafe {
                let event_fd = evdi_get_event_ready(self.evdi_handle);
                if event_fd != -1 {
                    self.handle_events();
                }
            }

            // Small delay to prevent busy waiting
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for DisplayLinkDriver {
    fn drop(&mut self) {
        println!("Shutting down DisplayLink driver...");

        // Disconnect from EVDI
        unsafe {
            evdi_disconnect(self.evdi_handle);
            evdi_close(self.evdi_handle);
        }

        // Release USB interface
        if let Ok(handle) = self.usb_handle.lock() {
            let _ = handle.release_interface(DISPLAY_INTERFACE);
        }
    }
}

fn main() {
    println!("DisplayLink Rust Driver v0.1.0");
    println!("========================================");

    // Initialize EVDI library
    unsafe {
        let mut version = evdi_lib_version {
            version_major: 0,
            version_minor: 0,
            version_patchlevel: 0,
        };
        evdi_get_lib_version(&mut version);
        println!("EVDI library version: {}.{}.{}",
            version.version_major, version.version_minor, version.version_patchlevel);
    }

    // Find and open DisplayLink USB device
    match rusb::Context::new() {
        Ok(mut context) => {
            println!("USB context initialized.");

            match find_displaylink_device(&mut context) {
                Some((device, device_desc)) => {
                    println!("DisplayLink device found!");
                    println!("  Bus: {}, Address: {}",
                        device.bus_number(), device.address());
                    println!("  VID: 0x{:04X}, PID: 0x{:04X}",
                        device_desc.vendor_id(), device_desc.product_id());

                    match device.open() {
                        Ok(handle) => {
                            println!("USB device opened successfully.");

                            // Create or open EVDI device
                            let evdi_handle = unsafe {
                                // Add new EVDI device
                                let card_no = evdi_add_device();
                                if card_no < 0 {
                                    eprintln!("Failed to add EVDI device");
                                    return;
                                }
                                println!("Created EVDI device: /dev/dri/card{}", card_no);

                                // Open the EVDI device
                                let handle = evdi_open(card_no);
                                if handle == EVDI_INVALID_HANDLE {
                                    eprintln!("Failed to open EVDI device");
                                    return;
                                }

                                // Connect with default EDID
                                println!("Connecting to EVDI with default EDID...");
                                evdi_connect(
                                    handle,
                                    DEFAULT_EDID.as_ptr(),
                                    DEFAULT_EDID.len() as u32,
                                    0, // No SKU area limit
                                );

                                // Enable cursor events
                                evdi_enable_cursor_events(handle, true);

                                handle
                            };

                            // Create driver instance
                            let mut driver = DisplayLinkDriver::new(evdi_handle, handle);

                            // Initialize USB device
                            match driver.initialize_device() {
                                Ok(_) => {
                                    println!("DisplayLink device initialized successfully.");
                                    println!("\nDriver is now running. Press Ctrl+C to exit.");

                                    // Run main event loop
                                    if let Err(e) = driver.run() {
                                        eprintln!("Driver error: {}", e);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Failed to initialize device: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error opening device: {}", e);
                        }
                    }
                }
                None => {
                    println!("DisplayLink device not found.");
                    println!("Please ensure the StarTech USB35DOCK is connected.");
                }
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
