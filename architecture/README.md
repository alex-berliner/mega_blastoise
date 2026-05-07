# Architecture Docs

This directory contains the architecture documentation for the Mega Blastoise workspace. For build instructions see [README.md](../README.md); for software internals see [TECHNICAL.md](../TECHNICAL.md).

## Reading Order

1. [System Overview](./01-system-overview.md)
2. [Core Battle Engine Flow](./02-core-battle-flow.md)
3. [Firmware Runtime (RP2040)](./03-firmware-runtime.md)
4. [Host Runtime and Test Harness](./04-host-runtime.md)
5. [Events and Input Contracts](./05-events-and-input.md)
6. [Memory and Debugging](./06-memory-and-debugging.md)
7. [Design Principles and Extension Guide](./07-design-principles.md)

## Quick Links

- Core crate: [`mega_blastoise_core`](../mega_blastoise_core/)
- Firmware crate: [`mega_blastoise_fw`](../mega_blastoise_fw/)
- Host/test crate: [`mega_blastoise_test`](../mega_blastoise_test/)
