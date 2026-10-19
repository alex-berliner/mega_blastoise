# Memory and Debugging

This page describes runtime diagnostics and troubleshooting flow.

## Heap Profiling

[`mem_profile.rs`](../../crates/mega_blastoise_fw/src/mem_profile.rs) provides:

- global heap allocator
- snapshot logging: `used`, `free`, `peak`, `total`

With the `mem-profile` feature, [`main.rs`](../../crates/mega_blastoise_fw/src/main.rs) emits a snapshot at boot.

## Runtime Tracing

Every console line the shared `DeviceRunner` produces goes to defmt (RTT) and
to the USB CDC console ([`usb_v2.rs`](../../crates/mega_blastoise_fw/src/subsystems/usb_v2.rs));
the `trace` feature adds core's own traces.

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
