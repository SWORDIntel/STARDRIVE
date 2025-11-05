use rusb::{Device, DeviceDescriptor, DeviceHandle, UsbContext};

// DisplayLink Vendor ID and Product ID
const DISPLAYLINK_VID: u16 = 0x17e9;
const DISPLAYLINK_PID: u16 = 0x4307;

fn main() {
    match rusb::Context::new() {
        Ok(mut context) => {
            println!("RUSB context initialized.");
            match find_displaylink_device(&mut context) {
                Some((mut device, device_desc)) => {
                    println!(
                        "DisplayLink device found! Bus: {}, Address: {}",
                        device.bus_number(),
                        device.address()
                    );
                    match device.open() {
                        Ok(mut handle) => {
                            println!("Device opened successfully.");
                            // Next steps will go here:
                            // 1. Detach kernel driver (if necessary)
                            // 2. Claim interface
                            // 3. Send/receive control/bulk transfers
                        }
                        Err(e) => {
                            eprintln!("Error opening device: {}", e)
                        }
                    }
                }
                None => {
                    println!("DisplayLink device not found.");
                }
            }
        }
        Err(e) => {
            eprintln!("Could not initialize RUSB context: {}", e)
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