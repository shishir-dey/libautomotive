<div align="center">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/libautomotive-dark.svg" width="1280">
  <source media="(prefers-color-scheme: light)" srcset="assets/libautomotive-light.svg" width="1280">
  <img alt="libautomotive logo" src="assets/libautomotive-light.svg" width="1280">
</picture>
</div>

A comprehensive Rust library for automotive protocol implementations, following the OSI layer model for clear separation of concerns. The library provides support for various automotive protocols including CAN, CAN-FD, ISO-TP, J1939, ISOBUS, UDS, and OBD-II.

## Features

- Complete automotive protocol stack implementation
- Modular and extensible design following OSI layers
- High-performance implementations
- Strong type safety and error handling
- Easy-to-use abstractions

### Supported Protocols

- **Physical Layer**: CAN and CAN-FD implementations
- **Data Link Layer**: Raw CAN frame handling
- **Network Layer**: J1939 and ISOBUS protocol implementations
- **Transport Layer**: ISO-TP (ISO 15765-2) implementation
- **Application Layer**: 
  - UDS (ISO 14229) implementation
  - OBD-II implementation
  - ISOBUS Diagnostic Protocol (DM1, DM2, DM3, DM11, DM13, DM22)

## Building

1. Ensure you have Rust and Cargo installed (https://rustup.rs/)
2. Clone the repository:
   ```bash
   git clone https://github.com/shishir-dey/libautomotive.git
   cd libautomotive
   ```
3. Build the library:
   ```bash
   cargo build
   ```
4. For release build:
   ```bash
   cargo build --release
   ```

## Testing

Run the test suite:
```bash
cargo test
```

For verbose test output:
```bash
cargo test -- --nocapture
```

## Example Usage

```rust
use libautomotive::application::{uds, obdii};
use libautomotive::transport::isobus_diagnostic;

// UDS example
let uds_config = uds::Config::default();
let uds_interface = uds::Interface::new(uds_config);

// OBD-II example
let obd_config = obdii::Config::default();
let obd_interface = obdii::Interface::new(obd_config);

// ISOBUS Diagnostic example
let mut isobus_diag = isobus_diagnostic::ISOBUSDiagnosticProtocol::new();
let dtc = isobus_diagnostic::DiagnosticTroubleCode::new(100, 1); // SPN: 100, FMI: 1
isobus_diag.add_dtc(dtc);
```

## License

MIT

## Credits and Acknowledgments

This library draws inspiration from and acknowledges the following open-source projects:

- [esp32-isotp-ble-bridge](https://github.com/bri3d/esp32-isotp-ble-bridge) - ESP32-IDF based BLE<->ISO-TP bridge
- [Open-SAE-J1939](https://github.com/DanielMartensson/Open-SAE-J1939) - Open source SAE J1939 implementation
- [uds-c](https://github.com/openxc/uds-c) - Unified Diagnostic Services (UDS) C library
- [obdii](https://github.com/ejvaughan/obdii) - OBD-II diagnostic protocol implementation
- [canis-can-sdk](https://github.com/kentindell/canis-can-sdk) - CAN protocol stack implementation
- [AgIsoStack++](https://github.com/Open-Agriculture/AgIsoStack-plus-plus) - Open-source C++ ISOBUS library
- [open-LIN-c](https://github.com/open-LIN/open-LIN-c) - Implementation of Local Interconnect Network in C
- [doip-library](https://github.com/doip/doip-library) - Diagnostic over IP (DoIP) protocol implementation