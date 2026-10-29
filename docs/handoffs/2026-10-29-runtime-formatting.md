# Handoff — runtime text formatting and mutable StringBuffer (2026-10-29)

Branch: `feat/lsp-mvp`

This document records the work after the LSP MVP handoff
([`docs/handoffs/2026-10-28-lsp-mvp.md`](../../docs/handoffs/2026-10-28-lsp-mvp.md)).
The formatting fix is committed at `05c75b29`; this handoff is committed
immediately after it. The branch was previously seven commits ahead of origin.

## Continue on another machine

```sh
git fetch origin
git switch feat/lsp-mvp
git pull --ff-only
cd src/rust
cargo test -q -p eudkit-core -p eudkit-ts
cargo build --release -q -p eudkit-cli
```

Build the user reproducer with:

```sh
cd ~/TestMap
/home/com/eudplib/src/rust/target/release/eudkit build -c eudkit.toml \
  -o /tmp/opencode/test-fixed1.scx
```

If `~/TestMap` is not present on the other machine, recreate a small map from
`packages/ts-sdk` and the focused script described below.

## What was broken

The user's `~/TestMap/test.js` showed none of the text produced by
`session.string(...)` / `StringBuffer.display()`, and
`f_eprintln("Current Wave: %d", wave)` always printed `0`. The map could also
appear blank with no triggers running.

Root causes found:

1. `FormatPrint`/`PrintAt` were lowered with compile-time constant
   substitution (`var_inits`). A runtime variable therefore became its initial
   value, usually `0`.
2. TypeScript `.printf`/`.format` could lower to `FormatString` and
   `DisplayTextId`, which the emitter silently ignored.
3. `StringBuffer.display()` with no arguments lowered to a static
   `DisplayText(template)` instead of showing the buffer's mutable contents.
4. Mutable buffers had no dedicated `STRx` storage. Reusing/deduplicating a
   string ID would make runtime writes corrupt other strings, and string-table
   insertion could invalidate addresses computed before all strings were
   registered.

## What changed

### Runtime formatting pass

New file: [`src/rust/eudkit-core/src/format_expand.rs`](../../src/rust/eudkit-core/src/format_expand.rs)

`expand_format_actions` runs in `lower_hir_to_lir` after reactive-rule lowering
and inlining, before CFG expansion. It:

- pre-registers all static `DisplayText` strings first, because
  `StringTable` may reuse empty slots and shift data offsets;
- allocates each mutable string buffer as
  `initial_text + NUL + reserved spaces`;
- allocates one shared 512-byte scratch string for formatted `DisplayText`
  calls;
- resolves every final string address only after all insertions;
- rewrites `FormatPrint`, `PrintAt`, `FormatStringBuffer`, and
  `DisplayStringBuffer` into primitive HIR: `If`, `While`, `MemRead`,
  `SetDeathsDynamic`/`SetMemoryDynamic`, variable assignment/arithmetic, and
  `DisplayTextId`;
- emits decimal and 8-digit hex digits at runtime, with NUL termination;
- uses multiplication by powers of two for byte placement because constant
  multiplication lowers to one conditional add per set bit.

### HIR and map context

- Added `StringBufferId`, `HirStringBuffer`, `DisplayStringBuffer`, and
  `FormatStringBuffer` in [`hir.rs`](../../src/rust/eudkit-core/src/hir.rs).
- Added `HirModule.string_buffers`, `add_string_buffer`, `string_buffer`, and
  `string_buffer_mut`.
- Added `StringTable::add_unique`, `MapData::add_unique_string`,
  `MapContext::add_unique_string`, and `MapContext::string_address` in
  [`map_context.rs`](../../src/rust/eudkit-core/src/map_context.rs).
  `add_unique` deliberately bypasses `stringmap` deduplication so mutable
  buffers never share storage.
- The old `FormatPrint`/`PrintAt` emission paths in
  [`trigger_emitter.rs`](../../src/rust/eudkit-core/src/lir/trigger_emitter.rs)
  are now unreachable and guarded.

### TypeScript lowering

[`compiler.rs`](../../src/rust/eudkit-ts/src/compiler.rs):

- creates real `HirStringBuffer`s for `session.string(...)`;
- tracks `MapString`s and emits `DisplayStringBuffer` for `.display()` with no
  arguments;
- handles `.format()`, `.printf()`, `.display_at()`, explicit format templates,
  and raw `DisplayTextId`;
- no longer drops unsupported formatting actions silently.

## Verification performed

| Check | Result |
| --- | --- |
| `cargo check -q -p eudkit-core -p eudkit-ts` | Pass |
| `cargo fmt -p eudkit-core -p eudkit-ts -- --check` | Pass |
| `cargo test -q -p eudkit-core -p eudkit-ts` | Pass after replacing the old CPString-residue assertion |
| Focused build | `/tmp/opencode/fmttest/out.scx` builds: `triggers=1392`, `payload_bytes=683720`, `entry=0x19195740` |
| User reproducer HIR | Contains `DisplayStringBuffer(@strbuf0)`; LIR display action is `str=9` |
| `format_print_test` | Now asserts dynamic EPD writes and a surviving `type=9` display action, not the old CPString stream |

The focused test script lives at `/tmp/opencode/fmttest/main.js`; its config and
generated map are in the same directory. `/tmp/opencode/test-fixed1.scx` was
built from the user reproducer before the latest test-only formatting cleanup.

## Known problems / not yet verified

1. **Severe trigger and payload bloat.** The focused map grows to 1392 triggers
   and 683,720 payload bytes. The LIR dump for the small test contains 8,344
   `type=45` actions. Decimal formatting currently unrolls nine `While` loops
   plus a units loop per formatted value, and each dynamic byte write expands
   into conditional alignment branches. This is the main remaining
   optimization target.
2. **No in-game verification.** There is no StarCraft client or faithful
   trigger-chain simulator locally. The generated `.scx` builds, but the user
   has not yet confirmed that the text appears or that the trigger chain runs.
3. **Original blank-map symptom is not conclusively explained.** The formatting
   fix removes silent drops and zero values, but the map that goes blank with no
   triggers executing may have an independent cause (payload size, trigger
   layout, string relocation, or bootstrap/next-pointer corruption). Do not
   assume the formatting fix alone resolves it.
4. **`format_print_test` is a structural regression test**, not a byte-exact
   emitter test. It verifies dynamic EPD writes and a display action survive
   expansion.
5. No issue-tracker tickets were created, per the user's instruction.

## Suggested next steps

1. **Profile the bloat.** Start with `format_expand.rs`. Determine whether the
   decimal digit loops or the dynamic-byte alignment branches dominate. A
   compact decimal formatter that writes from the least-significant digit, or
   a shared formatter routine, would remove most duplicated HIR. Consider
   writing the whole decimal number into a fixed-width field and then trimming
   leading spaces rather than nine subtraction loops.
2. **Verify in-game.** Ask the user to run the focused `.scx` and report
   whether "Wave N" shows a changing N, whether static text appears, and whether
   the map remains responsive. If it is blank, inspect the bootstrap trigger
   chain and the first payload trigger's `nextptr` before changing formatting
   again.
3. **Add focused regressions** for:
   - `%d` with a runtime `VarId` (not just `Const`);
   - `%x` with a runtime `VarId`;
   - `StringBuffer.format(...).display()` writing to its own `STRx` storage;
   - two buffers with identical initial text receiving independent writes;
   - `DisplayStringBuffer` mapping to the buffer's `string_id`;
   - string-table insertion that reuses an empty slot not invalidating a
     previously computed address.
4. **Check the TypeScript API contract** against
   [`packages/ts-sdk/src/index.ts`](../../packages/ts-sdk/src/index.ts) and
   [`GLOSSARY.md`](../../GLOSSARY.md): `.format()` mutates silently;
   `.display(*args)` formats and displays; `.display_at(line, *args)` formats to
   a screen line; `.display()` with no args shows current contents.
5. **Rebuild and inspect** the user map after any formatter change:
   ```sh
   cd /home/com/eudplib/src/rust
   cargo build --release -q -p eudkit-cli
   cd ~/TestMap
   /home/com/eudplib/src/rust/target/release/eudkit build -c eudkit.toml \
     -o /tmp/opencode/test-fixed1.scx
   ```
   Use `eudkit dump-ir -c eudkit.toml --hir-only` and `--lir-only` to inspect
   the expansion.

## Relevant files

- `src/rust/eudkit-core/src/format_expand.rs` — runtime formatting/buffer
  expansion; main optimization target.
- `src/rust/eudkit-core/src/hir.rs` — new HIR variants and string buffers.
- `src/rust/eudkit-core/src/map_context.rs` — unique string insertion and
  address resolution.
- `src/rust/eudkit-core/src/lir/mod.rs` — pass ordering.
- `src/rust/eudkit-core/src/lir/cfg_expansion.rs` — dynamic-write lowering and
  exhaustive action handling.
- `src/rust/eudkit-core/src/lir/trigger_emitter.rs` — old formatting paths now
  unreachable.
- `src/rust/eudkit-ts/src/compiler.rs` — TypeScript buffer/map-string lowering.
- `src/rust/eudkit-core/tests/format_print_test.rs` — updated structural test.
- `src/eudplib/string/strbuffer.py` — canonical Python StringBuffer behavior.
- `src/eudplib/string/cpprint.py` — reference runtime decimal formatter.
- `packages/ts-sdk/src/index.ts` — `MapString`/`StringBuffer` API.
- `~/TestMap/test.js` — user reproducer.
- `/tmp/opencode/fmttest/main.js` — focused formatting script.
- `/tmp/opencode/fmttest/out.scx` — focused generated map.
- `/tmp/opencode/fmt-test-dump.txt` — captured LIR dump used to inspect the
  expansion.

## Local-only files

Do not commit these unless the user asks:

- `EWait.py`
- `EWait.py:Zone.Identifier`
- `src/rust/eudkit-core/examples/` — ad hoc `dump_chk.rs` and
  `dump_strings.rs` diagnostics created during this investigation.

`docs/todo/display-resource-ewait.md` remains the deferred EWait design record;
no EWait work was done here.

## Suggested skills

Load these with the Skill tool when starting the relevant work:

- `diagnosing-bugs` — for the remaining blank-map/no-trigger symptom and any
  in-game failure.
- `code-review` — before merging the runtime formatting change.
- `research` — only if a StarCraft/CHK behavior needs verification against
  primary sources; local simulation is unavailable.
