# Trigger Loop Timing Model and Non-Blocking Reactive Rule Semantics

`eudkit` models all time-dependent and reactive trigger execution rules (`when`, `until`, `after`, `every`, `once`) strictly around StarCraft's native **Trigger Loop** evaluation cycle rather than real-world wall-clock milliseconds or engine `Wait(ms)` actions.

## Context

StarCraft Brood War's internal game engine advances in discrete logical frames (~24 ticks per second on 'Fastest' game speed, or ~42ms per tick). The trigger virtual machine does not evaluate triggers on every game frame by default. A 4-byte **Trigger Execution Timer** at `0x6509A0` counts down each game tick; when it reaches zero, the engine runs a Trigger Loop and reloads the timer to 30. In standard vanilla maps, triggers therefore evaluate roughly once every 30 to 31 game frames (a **Trigger Loop**, approximately 1.25–1.3 seconds at Fastest).

`Wait` uses a separate mechanism. Each player has a 4-byte wait timer in the eight-entry array at `0x650980`. A `Wait(ms)` action stops that player's trigger list at the wait and prevents later triggers in the same list from running until the timer expires. Traditional hyper triggers built from cyclic `Wait(0)` actions make the trigger loop run every other game frame (about 12 loops per second at Fastest). Single-tick EUD turbo triggers instead hold the Trigger Execution Timer at zero, making the loop run every game frame (about 24 loops per second at Fastest).

Legacy mapmakers historically attempted to use native `Wait(milliseconds)` actions for delays and intervals. In practice, native `Wait` causes critical failure modes:
1. **Wait Blocks**: `Wait` suspends the remainder of the executing player's trigger list at `0x650980`, so later triggers in that list do not run until the wait expires.
2. **Game-Speed Coupling**: Wait durations are milliseconds converted to game ticks using the current game speed, so the same script consumes different tick counts on different speeds.
3. **Unpredictable Scheduling**: Wall-clock milliseconds do not correspond to a fixed number of Trigger Loops, so wait-based delays are not a stable unit for map logic.

## Decision

1. **Rejection of Native `Wait`**: `eudkit` never emits native `Wait` actions for control flow, timers, or periodic callbacks.
2. **Trigger Loop as Fundamental Unit**: All time-based DSL APIs (`after(loops)`, `every(loops)`) accept durations strictly in integer counts of **Trigger Loops**:
   - In standard maps: 1 loop corresponds to 1 trigger evaluation cycle (roughly 30-31 frames at Fastest).
   - With traditional `Wait(0)` hyper triggers: 1 loop corresponds to 2 game ticks (about 12 loops per second at Fastest).
   - With single-tick EUD turbo triggers: 1 loop corresponds to 1 game tick (about 24 loops per second at Fastest).
3. **Non-Blocking Counter Lowering**: Reactive primitives lower to deterministic 4-byte memory variables (counters and guards) manipulated via native trigger conditions and actions:
   - `when(conditions)`: Evaluated every trigger loop; executes while conditions are satisfied.
   - `until(condition)`: Evaluates every trigger loop; executes on each pass while `condition` is false; when it becomes true, sets a 4B guard permanently to 1.
   - `after(loops)`: Decrements a 4B counter variable on each trigger loop; once reaching 0, sets a 4B completion guard and executes the body once.
   - `every(loops)`: Decrements a 4B counter variable on each trigger loop; upon reaching 0, resets the counter to `loops` and executes the body periodically.
   - `once(conditions?)`: Executes on the first trigger loop pass where `conditions` hold (or immediately when no conditions are given), setting a 4B guard to 0 permanently.

## Consequences

- **Multiplayer Safety**: Because all counters are driven by frame/loop evaluation rather than engine sleep threads, state transitions are deterministic across all players.
- **Zero Queue Stalling**: Other triggers continue executing uninterrupted without waiting on stalled threads.
- **Predictable Portability**: The same code has identical semantics whether a map uses standard pacing, traditional hyper triggers, or single-tick EUD turbo triggers; only the wall-clock interval represented by one loop changes.
