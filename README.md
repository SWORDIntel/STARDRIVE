# STARDRIVE: Rust-based DisplayLink Linux Driver

This repository contains a work-in-progress project to develop an open-source Linux driver for DisplayLink USB docks, written in Rust.

## Motivation
Official DisplayLink drivers for Linux are often proprietary, tied to specific kernel versions, and can be challenging to maintain across various distributions. This project aims to create a robust, open-source alternative that offers greater flexibility, stability, and community-driven development.

## Disclaimer
Developing a driver for a proprietary protocol like DisplayLink is a complex and long-term undertaking. This project involves reverse-engineering efforts and is not officially supported or endorsed by DisplayLink (Synaptics). Success is not guaranteed, and the process will require significant effort.

## Target Device
*   **Model:** StarTech USB35DOCK
*   **Vendor ID (VID):** `0x17e9`
*   **Product ID (PID):** `0x4307`

## Current Status
This project is currently in the early development and reverse-engineering phase. The core Rust project structure is set up, and initial integration with `libusb` (for USB communication) and `libevdi` (for virtual display management) has been established.

**Static analysis of Windows driver files has been completed.** This has provided valuable insights into the device's USB configuration and the different functionalities (display and network) handled by separate interfaces.

**Important Note:** Development is currently blocked by the need for a persistent Linux environment with properly installed kernel headers. The current environment (LiveCD) does not support kernel module compilation or persistent driver installation.

## High-Level Plan
1.  **Setup & Device Discovery:** (Completed)
    *   Identified device VID/PID.
    *   Rust project initialized with `rusb`.
    *   Basic device detection implemented.
    *   `udev` rules for permissions established.
2.  **EVDI Integration:** (Partially Completed)
    *   `libevdi` compiled from source.
    *   Rust bindings for `libevdi` generated.
    *   Integration confirmed via `cargo build`.
3.  **Windows Driver Analysis:** (Completed)
    *   Extracted strings from `dlidusb*.dll` files.
    *   Analyzed `dlidusb.inf`: Identified a comprehensive list of supported DisplayLink devices (VID `0x17e9`) and confirmed `PID_0x4307` for the StarTech USB35DOCK. The `MI_00` suffix likely indicates the display interface.
    *   Analyzed `dlcdcncm.inf`: Identified this as the network adapter driver, also supporting `VID_0x17e9` and `PID_0x4307` with an `MI_05` suffix, indicating the network interface.
4.  **USB Protocol Reverse Engineering:** (Pending - requires persistent Linux environment)
    *   **Goal:** Intercept `DisplayLinkManager`'s `libusb` calls to understand the proprietary protocol for device initialization, mode setting, and framebuffer data transfer (including compression/encryption).
    *   **Method:** This will involve running the official `DisplayLinkManager` binary in a controlled environment and using tools like `strace` or a custom `LD_PRELOAD` library to log all USB control and bulk transfers. This step requires a functional `evdi` kernel module and a running `DisplayLinkManager` in a persistent Linux environment.
5.  **Implement Rust Driver:**
    *   Replicate device initialization and framebuffer transfer using `rusb` and `libevdi` bindings based on reverse-engineered protocol.
6.  **Refinement & Features:**
    *   Implement hot-plugging, resolution changes, cursor updates, and performance optimizations.

## Development Environment Setup

### Prerequisites
*   A persistent Linux installation (not a LiveCD).
*   Rust toolchain (latest stable).
*   `libusb-1.0-0-dev` (or equivalent for your distribution).
*   `libdrm-dev` (or equivalent for your distribution).
*   Kernel headers for your currently running kernel (`linux-headers-$(uname -r)`).
*   `dkms` (for EVDI kernel module compilation).
*   `bindgen` (for generating Rust FFI bindings).

### Getting Started
1.  Clone the repository:
    ```bash
    git clone https://github.com/SWORDIntel/STARDRIVE.git
    cd STARDRIVE
    ```
2.  Ensure kernel headers and `libdrm-dev` are installed (see Prerequisites).
3.  Compile `libevdi`:
    ```bash
    cd evdi_source/library
    make
    cd ../..
    ```
4.  Build the Rust project:
    ```bash
    cargo build
    ```

## Contributing
Contributions are welcome! Please refer to the high-level plan for current development focus. Feel free to open issues or pull requests.

## License
This project is licensed under the [MIT License](LICENSE). (Note: A `LICENSE` file will be added later.)
