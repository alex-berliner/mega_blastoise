# Firmware interactions (mega_blastoise_fw)

How the RP2040 firmware's pieces talk to each other. Three layers:

1. **Spawned embassy tasks** (`subsystems/`): OLED, LED strips, buzzer, USB
   device. Each owns its hardware and drains a static command channel, so the
   game logic never blocks on IO.
2. **The main task** (`main.rs`): an endless `lobby -> battle -> lobby` loop.
3. **Shared core** (`mega-blastoise-core`): ALL display logic (`OledCmd` ->
   `OledController` -> `render_*`) and ALL input semantics (`ReadySequence`,
   `ChoiceCollector`) live here, identical to the web build (zero-drift rule).
   The firmware contributes raw IO only: matrix scanning, CDC bytes, I2C/PIO/PWM.

## Task and channel topology

```mermaid
flowchart TB
    subgraph HW[hardware]
        BT[button matrix]
        HOST[USB host]
        SSD[2x SSD1306]
        WS[2x WS2812 strip]
        PZ[piezo GP21]
    end

    subgraph MAIN[main task: lobby / battle loop]
        BTN["PicoBattleInput + PadScan<br/>(scan, debounce, tap/hold/chord)"]
        UIN["UsbBattleInput<br/>(line parser, narration out)"]
        CORE["core state machines<br/>ReadySequence / ChoiceCollector"]
        RUN["core::run_battle<br/>(battle_loop + BoardEventQueue)"]
        ENG[gen1_battle engine]
        EFX["BattleEffects<br/>(BoardEffects impl + anim delays)"]
    end

    subgraph TASKS[spawned tasks]
        UT[usb device task]
        OT["oled::task<br/>OledController + render_screen"]
        LT["led::task<br/>PIO0 + DMA"]
        BZ["buzzer::task<br/>PWM2B"]
    end

    BT --> BTN
    HOST <--> UT <--> UIN
    BTN -- PadEvent --> CORE
    UIN -- typed lines --> CORE
    CORE -- "InputBus.choices (cap 4)" --> RUN
    RUN -- "InputBus.prompt (cap 2)" --> CORE
    RUN <--> ENG
    RUN -- BoardEvent --> EFX
    EFX -- "narration: InputBus.log (cap 32)" --> UIN
    CORE -- "OledCmd chan (cap 8)" --> OT
    EFX -- "OledCmd chan (cap 8)" --> OT
    EFX -- "LedCmd chan (cap 8)" --> LT
    EFX -- "BuzzerCmd chan (cap 4)" --> BZ
    OT -- I2C0/I2C1 --> SSD
    LT --> WS
    BZ --> PZ
```

The `OledCmd`/`LedCmd`/`BuzzerCmd` channels are fire-and-forget statics
(`subsystems::oled::send` etc.), callable from anywhere in the main task.
`InputBus` is the battle's rendezvous: the runner prompts, input sources answer,
effects narrate.

## Lobby: demo, ready-up, countdown

`run_lobby_inner` (lobby.rs) knows nothing about GPIO or USB; it sees
`LobbyEvent`s through the `LobbyInput` trait, whose USB+buttons impl does the
racing.

```mermaid
sequenceDiagram
    autonumber
    actor P as Players
    participant BTN as PicoBattleInput
    participant USB as UsbBattleInput
    participant L as lobby.rs
    participant SEQ as core::ReadySequence
    participant OT as oled::task
    participant LT as led::task
    participant BZ as buzzer::task

    Note over L: main calls run_lobby()
    L->>LT: LobbyIdle
    L->>OT: LobbyState idle x2

    loop until a player or command interrupts
        L->>L: 15s idle countdown, then run_demo_battle<br/>(AI vs AI on a private InputBus, LEDs suppressed)
    end

    alt button press
        P->>BTN: press / long-press
        BTN-->>L: LobbyPress P1 / P2 / Long
    else USB command
        USB-->>L: LobbyUsbCmd (:ready, :vs-ai, :team ...)
    end
    L->>SEQ: seed event (pad_event / request_ai_opponent / ai_preset)

    loop drive_ready: select3(usb line, pad event, tick)
        P->>BTN: taps, holds, 4-corner chord
        BTN-->>SEQ: PadEvent (TapMove, HoldSwitch, Chord4 ...)
        USB-->>SEQ: typed_line / lobby cmds
        SEQ-->>OT: fx: picker, READY screen, 6v6 flash
        SEQ-->>LT: LobbyWaiting(ready flags)
        SEQ-->>USB: fx: ok/err lines, ready status
        Note over SEQ: per player: Idle -> picker -> READY<br/>(6v6 armed: press readies directly, no picker)
    end
    Note over SEQ: both READY, 1s unready grace elapsed

    SEQ-->>L: take() = ai_players + modes<br/>(6v6 forces concealed x2)
    L->>BZ: countdown beeps
    L->>USB: 3... 2... 1... GO!
    L->>LT: LobbyCountdown
    L-->>L: return LobbyResult to main
```

Main then seeds the RNG, draws the two teams (3 mons, or 6 when the chord
armed 6v6), uploads them to a fresh engine, sends `TeamInit` to the LEDs, and
hands `UsbBattleInput` the AI flags and control modes for the battle.

## Battle: one turn

`run_battle` races `battle_loop` against the input future
(`BattleController::run`, which is `usb.run_inner(bus, Some(buttons))`; the
`ChoiceCollector` inside owns menus, concealed scatter, and AI think timers).

```mermaid
sequenceDiagram
    autonumber
    actor P as Players
    participant BTN as PicoBattleInput
    participant HOST as USB host
    participant COL as ChoiceCollector<br/>(in usb.run_inner)
    participant BUS as InputBus
    participant RUN as core::run_battle
    participant ENG as gen1_battle
    participant EFX as BattleEffects
    participant OT as oled::task
    participant LT as led::task
    participant BZ as buzzer::task

    ENG-->>RUN: request (choose actions)
    RUN->>BUS: prompt.send(ActivePrompt) per player
    BUS-->>COL: prompt.receive
    COL->>COL: build SlotOptions<br/>(concealed: scatter layout, AI: 2s think deadline)
    COL-->>OT: action select / menus

    par human input
        P->>BTN: tap / hold / release
        BTN-->>COL: PadEvent
        HOST-->>COL: typed line (m1, s2, :press ...)
        COL-->>OT: concealed menus, details, waiting screen
    and AI sides
        COL->>COL: tick reaches think deadline, auto-pick
    end

    COL->>BUS: choices.send(PlayerChoice) per player
    BUS-->>RUN: all choices in
    RUN->>ENG: dispatch choices, turn resolves
    ENG-->>RUN: board log lines (move, crit, supereffective,<br/>damage, faint, win ...)

    loop each BoardEvent (via BoardEventQueue parse)
        RUN->>EFX: on_event
        EFX->>OT: oled_cmds_for_event (move-used + icon flicker,<br/>flashes, faint split, HP, speed badge)
        EFX->>LT: HpUpdate / Faint / Status / Win
        EFX->>BZ: Hit / Crit / SuperEffective / Win
        EFX->>EFX: hold anim delay (2500ms, win 7500ms)
        EFX->>BUS: log.try_send(narration)
        BUS-->>HOST: usb drains narration lines
    end

    Note over ENG: faint mid-turn: engine pauses,<br/>forced-switch request loops back to the prompt flow
    Note over RUN: on win/tie run_battle returns,<br/>main waits 4s and re-enters the lobby
```

The oled task itself is the last hop: it applies each `OledCmd` to the shared
`OledController`, re-renders the affected player's `Screen` into a framebuffer,
flushes over I2C, and mirrors into a shadow framebuffer for `oledfb|` dumps
(RTT/USB) used by headless testing.
