# Architecture Docs

This directory contains the architecture documentation for the Mega Blastoise workspace. For build instructions see [README.md](../../README.md); for software internals see [TECHNICAL.md](../TECHNICAL.md).

## Reading Order

1. [System Overview](./01-system-overview.md)
2. [Core Battle Engine Flow](./02-core-battle-flow.md)
3. [Firmware Runtime (RP2350)](./03-firmware-runtime.md)
4. [Host Runtime and Test Harness](./04-host-runtime.md)
5. [Events and Input Contracts](./05-events-and-input.md)
6. [Memory and Debugging](./06-memory-and-debugging.md)
7. [Design Principles and Extension Guide](./07-design-principles.md)
8. [Feature Map: Core vs. Target-Specific](./08-feature-map.md)
9. [Single-Screen Redesign](../archive/architecture/09-single-screen.md) (archived, superseded: the current design is [DESIGN.md](../../DESIGN.md))
10. [Production Board](../archive/architecture/10-production-board.md) (archived, superseded: see [DESIGN.md](../../DESIGN.md) and [hardware/ref_design/](../../hardware/ref_design/README.md))

## Quick Links

- Core crate: [`mega_blastoise_core`](../../crates/mega_blastoise_core/)
- Firmware crate: [`mega_blastoise_fw`](../../crates/mega_blastoise_fw/)
- Host/test crate: [`mega_blastoise_test`](../../crates/mega_blastoise_test/)
