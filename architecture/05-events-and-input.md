# Events and Input Contracts

This page documents the primary contracts connecting core and platform code.

## `BoardEvent` Contract

[`BoardEvent`](../mega_blastoise_core/src/board_event.rs) is the typed event model parsed from battler logs and injected prompts.

Examples:

- battle state: `BattleStart`, `Turn`, `Win`, `Tie`
- battle actions: `Move`, `SwitchIn`, `SwitchOut`, `Damage`, `Heal`, `Faint`
- control/pipeline: `Split`, `Prompt`

The event enum is the source of truth for board behavior.

## `BoardEffects` Contract

Platform code implements:

- `fn on_event(&mut self, event: BoardEvent)`

This is where event-to-output mapping occurs (logs today, hardware effects later).

## `BattleInput` Contract

Platform code implements:

- `async fn read_choice(&mut self, player_id: &str, request: &Request) -> String`

Returned strings must match battler command syntax (`move N`, `switch N`, etc.).

## Event Queue Semantics

[`BoardEventQueue`](../mega_blastoise_core/src/battle_effects.rs):

- buffers events in FIFO order
- handles `split` behavior where private/public rows occur in sequence
- dispatches events in order to a `BoardEffects` sink

Continue with [Memory and Debugging](./06-memory-and-debugging.md).
