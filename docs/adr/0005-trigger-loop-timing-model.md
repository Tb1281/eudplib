# Trigger Loop Timing Model and Non-Blocking Reactive Rule Semantics

`eudkit` models all time-dependent and reactive trigger execution rules (`when`, `until`, `after`, `every`, `once`) strictly around StarCraft's native **Trigger Loop** evaluation cycle rather than real-world wall-clock milliseconds or engine `Wait(ms)` actions.

## Context

StarCraft Brood War's internal game engine advances in discrete logical frames (~24 ticks per second on 'Fastest' game speed, or ~42ms per tick). However, the engine's trigger virtual machine does not evaluate triggers on every game frame by default. Instead, it decrements an internal countdown byte in memory (`0x6509A0` / `0x650980`). In standard vanilla maps, triggers evaluate roughly once every ~30 to 31 game frames (a **Trigger Loop**, approximately 1.25–1.3 seconds).

In competitive and complex custom maps, mapmakers frequently employ **EUD Turbo Triggers** or Hyper Triggers (via cyclic `Wait(0)` threads) that reset this countdown byte to 0 every tick, forcing the trigger loop to evaluate on **every single game frame** (24 times per second).

Legacy mapmakers historically attempted to use native `Wait(milliseconds)` actions for delays and intervals. In practice, native `Wait` causes critical failure modes:
1. **Thread Freezing**: `Wait` halts the executing player's trigger evaluation thread in `0x650980`, preventing all subsequent triggers in that player's chain from executing until the wait expires.
2. **Multiplayer Desynchronization**: Under network latency jitter and client CPU differences, wait countdown timers drift across clients, leading to game desyncs and drops.
3. **Speed Inconsistency**: Wall-clock milliseconds bear no fixed relation to game ticks; game speed settings ('Slow' through 'Fastest') and lag screens alter execution rate unpredictably.

## Decision

1. **Rejection of Native `Wait`**: `eudkit` never emits native `Wait` actions for control flow, timers, or periodic callbacks.
2. **Trigger Loop as Fundamental Unit**: All time-based DSL APIs (`after(loops)`, `every(loops)`) accept durations strictly in integer counts of **Trigger Loops**:
   - In standard maps: 1 loop corresponds to 1 trigger evaluation cycle (~31 frames).
   - In EUD Turbo Trigger maps: 1 loop corresponds to 1 game tick (1/24th second on Fastest).
3. **Non-Blocking Counter Lowering**: Reactive primitives lower to deterministic 4-byte memory variables (counters and guards) manipulated via native trigger conditions and actions:
   - `when(conditions)`: Evaluated every trigger loop; executes while conditions are satisfied.
   - `until(condition)`: Evaluates every trigger loop; executes until `condition` becomes true, then sets a 4B guard permanently to 1.
   - `after(loops)`: Decrements a 4B counter variable on each trigger loop; once reaching 0, sets a 4B completion guard and executes the body once.
   - `every(loops)`: Decrements a 4B counter variable on each trigger loop; upon reaching 0, resets the counter to `loops` and executes the body periodically.
   - `once(conditions?)`: Executes on the first trigger loop where `conditions` hold (or loop 0 if None), setting a 4B guard to 0 permanently.

## Consequences

- **Multiplayer Safety**: Because all counters are driven by frame/loop evaluation rather than engine sleep threads, state transitions are 100% synchronized and deterministic across all players.
- **Zero Queue Stalling**: Other triggers continue executing uninterrupted without waiting on stalled threads.
- **Predictable Portability**: The same code runs identically whether a map uses standard pacing or 24Hz Turbo Triggers, with loop counts translating directly into trigger evaluation cycles.
