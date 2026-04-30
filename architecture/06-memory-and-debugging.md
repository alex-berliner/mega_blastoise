# Memory and Debugging

This page describes runtime diagnostics and troubleshooting flow.

## Heap Profiling

[`mem_profile.rs`](../mega_blastoise_fw/src/mem_profile.rs) provides:

- global heap allocator
- snapshot logging: `used`, `free`, `peak`, `total`

[`main.rs`](../mega_blastoise_fw/src/main.rs) and [`usb_input.rs`](../mega_blastoise_fw/src/usb_input.rs) emit snapshots at key points:

- boot/init checkpoints
- battle setup checkpoints
- prompt entry/exit checkpoints
- per-turn checkpoints (if enabled)

## Runtime Tracing

Current firmware logs include:

- typed event traces in [`board_effects.rs`](../mega_blastoise_fw/src/board_effects.rs)
- request and accepted-choice traces in [`usb_input.rs`](../mega_blastoise_fw/src/usb_input.rs)

This makes it easier to correlate:

1. what the battle engine requested,
2. what user input was accepted,
3. which events were emitted, and
4. where heap spikes happened.

## Recommended Debug Workflow

1. Reproduce quickly on host when possible.
2. On firmware, read heap snapshots near crash point.
3. Use input/event traces to identify exact stage of failure.
4. Reduce peak allocations in the implicated stage.

Continue with [Design Principles and Extension Guide](./07-design-principles.md).
