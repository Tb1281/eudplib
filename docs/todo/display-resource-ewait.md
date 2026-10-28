# TODO — Display resource without EWait (deferred)

Status: **deferred** (2026-10-28). No StarCraft client is available for runtime
testing, and EWait has been dropped as the mechanism. Do not implement this
until the LSP/eudkit work is complete and a 32-bit and/or 64-bit SC:R client is
available for a probe.

## Decision

- 64-bit support is required. EWait is not viable on 64-bit with the information
  available, so **do not port EWait**.
- Do not add a public `ewait_write` primitive.
- Do not change `display_at` or add a `display_resource` API until the
  persistence questions below are answered on a real client.

## What EWait does

`EWait.py` (untracked local reference, version 0.0.33, `ignore64 = True`) is a
32-bit real-memory write primitive. It does not use an EUD `SetMemory` to write
the resource line.

- `Address_WAIT = 0xDB8714`, `Address_WAIT_FLAG = 0xDB870C`.
- `SetEWait(src, val)` sets CP (`0x6509B0`) to `(src - 0xDB8714) / 4` and runs a
  `Wait` action. The game writes `val` to `0xDB8714 + 4*CP`, i.e. to `src`.
- `GetFlagAddr` maps the same index to the byte flag array at `0xDB870C`.
- `init_resource_line()` calls `SetEWait(0xD7DCD8, 0x80000000)`, then writes
  `0x0202` at the base of HUD line 11.
- `f_displayResourceText()` writes formatted text at line 11 + 2.
- `f_expel64Users()` detects 64-bit clients and ejects them.

## Why 64-bit EWait is blocked

- `0xDB8714` and `0xD7DCD8` are SC:R 32-bit process addresses, not 1.16.1 EUD
  addresses. The documented 1.16.1 offset table ends at `0x006D5EC8`.
- The 64-bit Wait timer/flag array addresses and the 64-bit resource-line
  freeze variable are undocumented. There is no public 64-bit offset table.
- No 64-bit SC:R client or binary is available in this environment to probe or
  reverse engineer.
- The Wait-CP trick depends on a 32-bit index into a real array. It is unknown
  whether the 64-bit client sign-extends the index, validates it, or keeps the
  target within reach.
- Conclusion: 64-bit EWait would be a reverse-engineering project, not a port.

## No-EWait options to probe

1. **Direct line-11 write + refresh.** Write at
   `0x640B60 + 218*11 + 2` through the EUD emulator, preserving the `0x0202`
   prefix. Re-write every trigger loop, every N loops, or when the displayed
   value changes. This is the closest to `f_displayResourceText` but cannot
   disable the native resource-line updater.
2. **Native `DisplayText` action with `0x640B58 = 11`.** Set the next display
   text line to 11, emit a `DisplayText` action for the formatted string, then
   restore `0x640B58`. This uses the game's own display-text path and may be
   more 64-bit-safe than a direct buffer write.
3. **Dedicated HUD line.** Use line 10 or 12 instead of the resource line, so
   the native resource updater never competes. This changes the visual target
   but avoids the freeze problem and the line-11 conflict.

## Open questions (answer with a probe map)

- Does a direct write at line 11 + 2 persist? Does the native updater overwrite
  it every frame, only when minerals/gas/supply change, or never?
- Does the `0x0202` prefix matter? What happens if `display_at(11)` writes at
  the line base instead of +2?
- Does the native `DisplayText` path write line 11 correctly, and does it
  advance `0x640B58` in a way that interferes with chat?
- Does the behavior differ between 32-bit and 64-bit SC:R?
- Which line can be owned without breaking `texteffect.py` (line 11 scratch) or
  `locale.py` / `f_eprintln` (line 12)?
- What refresh cadence is acceptable: every trigger loop, hyper-trigger every
  frame, or only when the custom value changes?

## Probe plan

1. Build a test map that writes a distinctive string to line 11 at +2, with a
   visible counter that changes every trigger loop.
2. Run it on 32-bit and 64-bit SC:R with native resources static and changing.
3. Repeat with the native `DisplayText` path (`0x640B58 = 11`).
4. Record persistence, flicker, chat interference, and any EUD errors.
5. Only then choose direct write + refresh, native `DisplayText`, or a
   dedicated line. If none is acceptable, drop the feature.

## Revisit criteria

- #52 and the eudkit SDK documentation are complete.
- A 32-bit and 64-bit SC:R client is available for a probe.
- The persistence questions above are answered.

## References

- `EWait.py` — untracked local reference; do not commit it.
- `docs/handoffs/2026-10-28-lsp-mvp.md` — LSP status and earlier analysis.
- `src/eudkit/types.py` — `display_at`, `SetMemory`, `SetMemoryX`.
- `src/rust/eudkit-core/src/lir/trigger_emitter.rs` — display-buffer lowering at
  `0x640B60 + 218*line`.
- `src/eudplib/string/texteffect.py` — line 11 scratch use.
- `src/eudplib/string/locale.py`, `src/eudplib/string/cpprint.py` — line 12
  status/chat use.
- EUD Book:
  <https://armoha.github.io/eud-book/offsets/DisplayText.html>,
  <https://armoha.github.io/eud-book/offsets/NextDisplayTextLine.html>.
