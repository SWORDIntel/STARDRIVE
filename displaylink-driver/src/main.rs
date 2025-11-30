// Allow warnings for auto-generated bindings
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

mod displaylink_protocol;
mod driver;
mod evdi;
mod manager;
mod network_adapter;

use manager::DisplayLinkManager;
use std::env;

fn main() {
    println!("DisplayLink Rust Driver v0.3.0 - Modular Architecture");
    println!("=====================================================");
    println!("Features: Multi-monitor, Hot-plug, Power management, 60FPS, Dithering");
    println!();

    // Initialize EVDI library
    unsafe {
        let mut version = evdi::evdi_lib_version {
            version_major: 0,
            version_minor: 0,
            version_patchlevel: 0,
        };
        evdi::evdi_get_lib_version(&mut version);
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

// Re-export verbose_enabled for submodules
pub fn verbose_enabled() -> bool {
    *manager::VERBOSE_LOG.get_or_init(|| env::var("DISPLAYLINK_DRIVER_VERBOSE").is_ok())
}
