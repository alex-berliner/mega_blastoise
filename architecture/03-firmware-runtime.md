# Firmware Runtime (RP2040)

This page describes `mega_blastoise_fw`.

## Startup Sequence

[`main.rs`](../mega_blastoise_fw/src/main.rs) does the following:

1. Initialize RP2040 peripherals via `embassy_rp::init`.
2. Initialize heap allocator (`mem_profile::init_heap`).
3. Configure RTT/defmt output channels.
4. Build USB CDC device and spawn USB task.
5. Configure I2C0/I2C1 and spawn PN532 tasks.
6. Create battle state and start battle.
7. Run shared `run_battle` loop.

## Long-Lived Tasks

- `usb_task`: runs USB device stack forever.
- `pn532_task_i2c0` and `pn532_task_i2c1`: background reader loops.
- `main` async context: battle orchestration.

## Firmware Adapters

- [`usb_input.rs`](../mega_blastoise_fw/src/usb_input.rs)
  - implements `BattleInput` over USB CDC packets.
  - prompts for choices, validates user input, returns battler command strings.
- [`board_effects.rs`](../mega_blastoise_fw/src/board_effects.rs)
  - implements `BoardEffects`.
  - currently logs typed events through defmt.
  - future home for LEDs, buzzer, and physical board outputs.

Continue with [Host Runtime and Test Harness](./04-host-runtime.md).
