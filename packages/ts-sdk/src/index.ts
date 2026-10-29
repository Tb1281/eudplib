/**
 * @eudkit/sdk - First-class TypeScript SDK for StarCraft: Remastered EUD Mapmaking
 */

// Memory Addresses & Constants
export const DEATHS_BASE_ADDR = 0x0058A364;
export const CURRENT_PLAYER_ADDR = 0x006509B0;
export const GAME_TICK_ADDR = 0x0057F23C;

/**
 * Computes the EPD (Extended Player Deaths) offset for a given StarCraft virtual memory address:
 * `EPD(x) = (x - 0x58A364) // 4`
 */
export function EPD(address: number): number {
  return Math.floor((address - DEATHS_BASE_ADDR) / 4);
}

// Player Constants
export const Player1 = 0;
export const Player2 = 1;
export const Player3 = 2;
export const Player4 = 3;
export const Player5 = 4;
export const Player6 = 5;
export const Player7 = 6;
export const Player8 = 7;
export const Player9 = 8;
export const Player10 = 9;
export const Player11 = 10;
export const Player12 = 11;

export const P1 = 0;
export const P2 = 1;
export const P3 = 2;
export const P4 = 3;
export const P5 = 4;
export const P6 = 5;
export const P7 = 6;
export const P8 = 7;
export const P9 = 8;
export const P10 = 9;
export const P11 = 10;
export const P12 = 11;

export const CurrentPlayer = 13;
export const Foes = 14;
export const Allies = 15;
export const NeutralPlayers = 16;
export const AllPlayers = 17;
export const Force1 = 18;
export const Force2 = 19;
export const Force3 = 20;
export const Force4 = 21;
export const NonAlliedVictoryPlayers = 26;

// Comparisons
export const AtLeast = 0;
export const AtMost = 1;
export const Exactly = 10;

// Modifiers
export const SetTo = 7;
export const Add = 8;
export const Subtract = 9;

// Resource Types
export const Ore = 0;
export const Gas = 1;
export const OreAndGas = 2;

// Switch States & Actions
export const Set = 2;
export const Cleared = 3;
export const Clear = 5;
export const Toggle = 6;
export const Random = 7;

// Unit Counts & Orders
export const All = 0;
export const Move = 0;
export const Patrol = 1;
export const Attack = 2;

// Score Types
export const Total = 0;
export const Units = 1;
export const Buildings = 2;
export const UnitsAndBuildings = 3;
export const Kills = 4;
export const Razings = 5;
export const KillsAndRazings = 6;
export const Custom = 7;

export type ExprLike = number | Var | MaskedVar | Expr;

/** Symbolic arithmetic/bitwise expression node compiled by the EUD AST compiler. */
export class Expr {
  constructor(public readonly raw: any) {}
}

/**
 * Handle to a mutable EUD variable.
 *
 * In compiled TypeScript the variable is lowered to a 4-byte memory cell or a
 * 72-byte VProc depending on liveness. In plain JS execution it holds `initVal`.
 */
export class Var {
  private static nextId = 0;
  public readonly id: number;

  constructor(public initVal: number = 0, public readonly name: string = "") {
    this.id = Var.nextId++;
  }

  // Value getter / setter for script ergonomics
  public get(): number {
    return this.initVal;
  }

  public set(val: number | ExprLike): void {
    if (typeof val === "number") {
      this.initVal = val;
    }
  }

  public mask(bitmask: number): MaskedVar {
    return new MaskedVar(this, bitmask);
  }

  public add(val: number | ExprLike): void {}
  public subtract(val: number | ExprLike): void {}
  public setTo(val: number | ExprLike): void {}

  public atLeast(val: number | ExprLike): Condition {
    return new Condition("VarAtLeast", [this, val]);
  }

  public atMost(val: number | ExprLike): Condition {
    return new Condition("VarAtMost", [this, val]);
  }

  public exactly(val: number | ExprLike): Condition {
    return new Condition("VarExactly", [this, val]);
  }

  public valueOf(): number {
    return this.initVal;
  }
}

/** Bitmasked view of a {@link Var}: comparisons and writes affect selected bits. */
export class MaskedVar {
  constructor(public readonly variable: Var, public readonly mask: number) {}
}

/**
 * Creates a top-level or local EUD variable.
 */
export function var_(initVal: number = 0, name: string = ""): Var {
  return new Var(initVal, name);
}
export { var_ as var };

// --- Conditions ---

/** StarCraft trigger condition node built by the stock condition helpers. */
export class Condition {
  constructor(public readonly typeName: string, public readonly args: any[]) {}
}

/** Condition that is always satisfied (StarCraft "Always"). */
export function Always(): Condition {
  return new Condition("Always", []);
}

/** Condition that is never satisfied (StarCraft "Never"). */
export function Never(): Condition {
  return new Condition("Never", []);
}

/** Condition that is true when the local player matches `CurrentPlayer`. */
export function IsUserCP(): Condition {
  return new Condition("IsUserCP", []);
}

/**
 * Condition on a player's death counter for one unit type.
 *
 * True when `player` has suffered `comparison` `amount` deaths of `unit`.
 * `comparison` is `AtLeast`, `AtMost`, or `Exactly`; `unit` is a unit ID or
 * map-context unit name.
 */
export function Deaths(
  player: ExprLike,
  comparison: number,
  amount: ExprLike,
  unit: number | string = 0
): Condition {
  return new Condition("Deaths", [player, comparison, amount, unit]);
}

/**
 * Masked variant of {@link Deaths} using EUD extended death counters.
 *
 * Compares `(deaths & mask) <comparison> amount`; `unit` is a unit ID or
 * map-context unit name.
 */
export function DeathsX(
  player: ExprLike,
  comparison: number,
  amount: ExprLike,
  unit: number | string = 0,
  mask: number = 0xFFFFFFFF
): Condition {
  return new Condition("DeathsX", [player, comparison, amount, unit, mask]);
}

/**
 * Condition on the 32-bit dword at an absolute virtual address.
 *
 * `address` is a raw virtual address, not an EPD.
 */
export function Memory(
  address: number,
  comparison: number,
  value: ExprLike
): Condition {
  return new Condition("Memory", [address, comparison, value]);
}

/**
 * Masked condition on the dword at an absolute virtual address.
 *
 * Compares `(memory[address] & mask) <comparison> value`.
 */
export function MemoryX(
  address: number,
  comparison: number,
  value: ExprLike,
  mask: number
): Condition {
  return new Condition("MemoryX", [address, comparison, value, mask]);
}

/**
 * Condition on a player's accumulated resources.
 *
 * True when `player` has `comparison` `amount` of `resource` (`Ore`, `Gas`, or
 * `OreAndGas`).
 */
export function Accumulate(
  player: number,
  comparison: number,
  amount: number,
  resource: number = Ore
): Condition {
  return new Condition("Accumulate", [player, comparison, amount, resource]);
}

/**
 * Condition on how many units a player commands.
 *
 * True when `player` commands `comparison` `amount` units of type `unit`.
 */
export function Command(
  player: number,
  comparison: number,
  amount: number,
  unit: number | string = 0
): Condition {
  return new Condition("Command", [player, comparison, amount, unit]);
}

/**
 * Condition on units a player brings to a location.
 *
 * True when `player` has `comparison` `amount` units of type `unit` inside
 * `location`. Both `unit` and `location` accept IDs or map-context names.
 */
export function Bring(
  player: number,
  comparison: number,
  amount: number,
  unit: number | string,
  location: number | string
): Condition {
  return new Condition("Bring", [player, comparison, amount, unit, location]);
}

/**
 * Condition on a switch's state.
 *
 * True when `switchId` (a switch ID or map-context switch name) is `Set` or
 * `Cleared`.
 */
export function Switch(
  switchId: number | string,
  state: number = Set
): Condition {
  return new Condition("Switch", [switchId, state]);
}

/**
 * Condition on elapsed scenario time.
 *
 * True when `comparison` `time` game seconds have elapsed since the scenario
 * started.
 */
export function ElapsedTime(comparison: number, time: number): Condition {
  return new Condition("ElapsedTime", [comparison, time]);
}

/**
 * Condition on one of a player's score counters.
 *
 * True when `player`'s `scoreType` counter (`Total`, `Units`, `Buildings`,
 * `Kills`, `Razings`, `KillsAndRazings`, `Custom`, ...) is `comparison`
 * `amount`.
 */
export function Score(
  player: number,
  comparison: number,
  amount: number,
  scoreType: number = 0
): Condition {
  return new Condition("Score", [player, comparison, amount, scoreType]);
}

// --- Actions ---

/** StarCraft trigger action node built by the stock action helpers. */
export class Action {
  constructor(public readonly typeName: string, public readonly args: any[]) {}
}

/**
 * Action that modifies a player's death counter for one unit type.
 *
 * Applies `modifier` (`SetTo`, `Add`, or `Subtract`) with `value` to `player`'s
 * death counter for `unit`. `unit` is a unit ID or map-context unit name.
 */
export function SetDeaths(
  player: ExprLike,
  modifier: number,
  value: ExprLike,
  unit: number | string = 0
): Action {
  return new Action("SetDeaths", [player, modifier, value, unit]);
}

/**
 * Masked variant of {@link SetDeaths} using EUD extended death counters.
 *
 * Applies `modifier` with `value` only to the counter bits selected by `mask`;
 * unselected bits are preserved.
 */
export function SetDeathsX(
  player: ExprLike,
  modifier: number,
  value: ExprLike,
  unit: number | string = 0,
  mask: number = 0xFFFFFFFF
): Action {
  return new Action("SetDeathsX", [player, modifier, value, unit, mask]);
}

/**
 * Action that modifies the 32-bit dword at an absolute virtual address.
 *
 * `address` is a raw virtual address, not an EPD.
 */
export function SetMemory(
  address: number,
  modifier: number,
  value: ExprLike
): Action {
  return new Action("SetMemory", [address, modifier, value]);
}

/**
 * Masked action that modifies the dword at an absolute virtual address.
 *
 * Applies `modifier` with `value` only to the bits selected by `mask`.
 */
export function SetMemoryX(
  address: number,
  modifier: number,
  value: ExprLike,
  mask: number
): Action {
  return new Action("SetMemoryX", [address, modifier, value, mask]);
}

/**
 * Action that modifies a player's resources.
 *
 * Applies `modifier` (`SetTo`, `Add`, or `Subtract`) with `amount` to `resource`
 * (`Ore`, `Gas`, or `OreAndGas`).
 */
export function SetResources(
  player: number,
  modifier: number,
  amount: number,
  resource: number = Ore
): Action {
  return new Action("SetResources", [player, modifier, amount, resource]);
}

/**
 * Displays a static string in the chat log.
 *
 * `alwaysDisplay` sets the action's always-display flag.
 */
export function DisplayText(text: string, alwaysDisplay: boolean = true): Action {
  return new Action("DisplayText", [text, alwaysDisplay]);
}

/**
 * Formats values and displays the result in the chat log.
 *
 * `template` uses `%d`/`%x`/`%i`/`%u`/`%p` placeholders matching `args`.
 */
export function displayText(template: string, ...args: ExprLike[]): Action {
  return new Action("displayText", [template, ...args]);
}

/** Formats values using printf-style placeholders and displays them (v1-compatible name). */
export function f_eprintln(template: string, ...args: ExprLike[]): Action {
  return new Action("f_eprintln", [template, ...args]);
}

/**
 * Displays text or formatted values on a fixed HUD line.
 *
 * `line` is the screen line index, 0 through 12.
 */
export function display_at(line: number, template: string, ...args: ExprLike[]): Action {
  return new Action("display_at", [line, template, ...args]);
}

export const displayAt = display_at;
export const DisplayTextAt = display_at;

/**
 * @deprecated Use `display_at` or `displayAt` instead.
 */
export const print_at = display_at;

/** Displays a map string by its STRx string ID in the chat log. */
export function DisplayTextId(stringId: number, alwaysDisplay: boolean = true): Action {
  return new Action("DisplayTextId", [stringId, alwaysDisplay]);
}

/**
 * Creates `count` units of a type at a location for a player.
 *
 * `unit` and `location` accept IDs or map-context names. Has no effect while
 * the game is paused.
 */
export function CreateUnit(
  count: number,
  unit: number | string,
  location: number | string,
  player: number
): Action {
  return new Action("CreateUnit", [count, unit, location, player]);
}

/** Adds a comment to the trigger, shown in the trigger editor. No runtime effect. */
export function Comment(text: string): Action {
  return new Action("Comment", [text]);
}

/** Keeps the enclosing trigger running on later loop passes. */
export function PreserveTrigger(): Action {
  return new Action("PreserveTrigger", []);
}

/** Ends the scenario in victory for the trigger owner. */
export function Victory(): Action {
  return new Action("Victory", []);
}

/** Ends the scenario in defeat for the trigger owner. */
export function Defeat(): Action {
  return new Action("Defeat", []);
}

/**
 * Kills all units of a type owned by a player.
 *
 * The SDK wrapper order is `KillUnit(unit, player)`. The underlying StarCraft
 * raw trigger action is `(player, unit, amount)`; `amount` is unused, and the
 * action affects every unit of `unit` owned by `player`.
 */
export function KillUnit(unit: number | string, player: number): Action {
  return new Action("KillUnit", [unit, player]);
}

/**
 * Kills up to `count` units of a type at a location for a player.
 *
 * `count` is `All` or a positive number; `unit` and `location` accept IDs or
 * map-context names. Unlike {@link KillUnit}, only units inside `location` are
 * affected, up to `count` of them.
 */
export function KillUnitAt(
  count: number,
  unit: number | string,
  location: number | string,
  player: number
): Action {
  return new Action("KillUnitAt", [count, unit, location, player]);
}

/**
 * Removes up to `count` units of a type at a location for a player.
 *
 * Works like {@link KillUnitAt}, but the units disappear without dying and
 * without incrementing death counters.
 */
export function RemoveUnitAt(
  count: number,
  unit: number | string,
  location: number | string,
  player: number
): Action {
  return new Action("RemoveUnitAt", [count, unit, location, player]);
}

/**
 * Centers a location on units of a type owned by a player.
 *
 * Moves `location` to center on an `onUnit` unit owned by `owner` inside the
 * search area `destLocation`. If none is found, `location` moves to the center
 * of `destLocation`.
 */
export function MoveLocation(
  location: number | string,
  onUnit: number | string,
  owner: number,
  destLocation: number | string
): Action {
  return new Action("MoveLocation", [location, onUnit, owner, destLocation]);
}

/**
 * Teleports up to `count` units of a type between locations.
 *
 * Moves `unit` units owned by `player` from `srcLocation` to `destLocation`.
 * `count` is `All` or a positive number.
 */
export function MoveUnit(
  count: number,
  unit: number | string,
  player: number,
  srcLocation: number | string,
  destLocation: number | string
): Action {
  return new Action("MoveUnit", [count, unit, player, srcLocation, destLocation]);
}

/**
 * Transfers units from one player to another.
 *
 * Gives up to `count` `unit` units owned by `player` inside `srcLocation` to
 * `toPlayer`. `count` is `All` or a positive number.
 */
export function GiveUnits(
  count: number,
  unit: number | string,
  player: number,
  srcLocation: number | string,
  toPlayer: number
): Action {
  return new Action("GiveUnits", [count, unit, player, srcLocation, toPlayer]);
}

/**
 * Orders units at a location to move, patrol, or attack a destination.
 *
 * Applies to `unit` units owned by `player` inside `srcLocation`. `orderType`
 * uses `Move`, `Patrol`, or `Attack`.
 */
export function Order(
  unit: number | string,
  player: number,
  srcLocation: number | string,
  orderType: number,
  destLocation: number | string
): Action {
  return new Action("Order", [unit, player, srcLocation, orderType, destLocation]);
}

/** Plays a WAV sound file bundled with the map. */
export function PlayWAV(wavFile: string): Action {
  return new Action("PlayWAV", [wavFile]);
}

/** Pings the minimap at `location` (a location ID or map-context name) for all players. */
export function MinimapPing(location: number | string): Action {
  return new Action("MinimapPing", [location]);
}

/**
 * Action that modifies one of a player's score counters.
 *
 * Applies `modifier` (`SetTo`, `Add`, or `Subtract`) with `amount` to
 * `scoreType` (`Total`, `Units`, `Buildings`, `Kills`, `Razings`, `Custom`, ...).
 */
export function SetScore(
  player: number,
  modifier: number,
  amount: number,
  scoreType: number = 0
): Action {
  return new Action("SetScore", [player, modifier, amount, scoreType]);
}

/**
 * Action that changes a switch's state.
 *
 * `switchId` is a switch ID or map-context switch name. `action` uses the
 * switch action constants: `Set`, `Clear`, `Toggle`, or `Random`.
 */
export function SetSwitch(switchId: number | string, action: number = Set): Action {
  return new Action("SetSwitch", [switchId, action]);
}

// --- Memory Read/Write Operations ---

/**
 * Reads a 32-bit dword from an absolute virtual address or Var pointer.
 *
 * Lowered by the EUD AST compiler; the plain JS runtime returns 0.
 */
export function dwread(ptr: number | Var): number { return 0; }
/** Reads a 32-bit dword from an EPD. Lowered by the compiler; runtime returns 0. */
export function dwread_epd(epd: number | Var): number { return 0; }
/** Reads a pointer at an EPD and converts it to an EPD. Runtime returns 0. */
export function epdread_epd(epd: number | Var): number { return 0; }
/** Reads an 8-bit byte from an absolute virtual address. Runtime returns 0. */
export function bread(ptr: number | Var): number { return 0; }
/** Reads an 8-bit byte from an EPD. Runtime returns 0. */
export function bread_epd(epd: number | Var): number { return 0; }
/** Reads a 16-bit word from an absolute virtual address. Runtime returns 0. */
export function wread(ptr: number | Var): number { return 0; }
/** Reads a 16-bit word from an EPD. Runtime returns 0. */
export function wread_epd(epd: number | Var): number { return 0; }

/** Writes a 32-bit dword to an absolute virtual address. */
export function dwwrite(ptr: number | Var, val: number | Var): Action {
  return SetMemory(typeof ptr === "number" ? ptr : 0, SetTo, val);
}
/** Writes a 32-bit dword at an EPD as a `SetDeaths` action. */
export function dwwrite_epd(epd: number | Var, val: number | Var): Action {
  return SetDeaths(epd, SetTo, val, 0);
}
/**
 * Writes a byte-width value to an absolute virtual address.
 *
 * The current SDK lowers this through a dword `SetMemory` action; use
 * {@link bwrite_epd} for an explicitly masked EPD write.
 */
export function bwrite(ptr: number | Var, val: number | Var): Action {
  return SetMemory(typeof ptr === "number" ? ptr : 0, SetTo, val);
}
/** Writes an 8-bit byte at an EPD as a `SetDeaths` action. */
export function bwrite_epd(epd: number | Var, val: number | Var): Action {
  return SetDeaths(epd, SetTo, val, 0);
}
/**
 * Writes a word-width value to an absolute virtual address.
 *
 * The current SDK lowers this through a dword `SetMemory` action; use
 * {@link wwrite_epd} for an explicitly masked EPD write.
 */
export function wwrite(ptr: number | Var, val: number | Var): Action {
  return SetMemory(typeof ptr === "number" ? ptr : 0, SetTo, val);
}
/** Writes a 16-bit word at an EPD as a `SetDeaths` action. */
export function wwrite_epd(epd: number | Var, val: number | Var): Action {
  return SetDeaths(epd, SetTo, val, 0);
}

/**
 * Emits action nodes into the current trigger block.
 *
 * The AST compiler inlines action calls; this function is a runtime no-op.
 */
export function DoActions(...actions: Action[]): void {
  // Runtime stub; AST lowering compiles action calls directly
}

// --- Control Flow & Metaprogramming ---

/**
 * Returns the integer sequence for a compile-time unrolled loop.
 *
 * `loopRange(stop)` iterates from 0; `loopRange(start, stop, step)` mirrors a
 * positive or negative Python `range`. `step` must be non-zero.
 */
export function loopRange(start: number, stop?: number, step: number = 1): number[] {
  const actualStart = stop === undefined ? 0 : start;
  const actualStop = stop === undefined ? start : stop;
  const result: number[] = [];
  for (let i = actualStart; step > 0 ? i < actualStop : i > actualStop; i += step) {
    result.push(i);
  }
  return result;
}

/** Options for the {@link fn} decorator. */
export interface FnOptions {
  mainLoop?: boolean;
  inline?: "auto" | "always" | "never";
}

/** Decorator factory that marks a function for EUDK compilation with options. */
export function fn(options?: FnOptions): (target: any) => any;
/** Marks a function for EUDK compilation with default lowering. */
export function fn(target: any): any;
/** Implementation of the {@link fn} overloads; returns the target or decorator unchanged. */
export function fn(targetOrOptions?: any): any {
  if (typeof targetOrOptions === "function") {
    return targetOrOptions;
  }
  return (target: any) => target;
}

export const eud = {
  fn,
  loopRange,
};

// --- Game Structs ---

/**
 * Typed handle to a StarCraft: Remastered CSprite structure.
 *
 * `epd` locates the structure in EPD space; `ptr` optionally stores the raw
 * pointer. Use {@link CSprite.cast} when the source is a pointer.
 */
export class CSprite {
  public ptr: number | Var;
  public epd: number | Var;

  constructor(epd: number | Var, ptr: number | Var = 0) {
    this.epd = epd;
    this.ptr = ptr;
  }

  public static cast(epd: number | Var, ptr: number | Var = 0): CSprite {
    return new CSprite(epd, ptr);
  }

  // Common CSprite offsets (relative to EPD)
  // +0x00A: spriteType (word) -> EPD + 2, bits 16..31
  // +0x00C: player (byte) -> EPD + 3, bits 0..7
  // +0x014: posX (word) -> EPD + 5, bits 0..15
  // +0x016: posY (word) -> EPD + 5, bits 16..31
  public get spriteType(): number { return 0; }
  public set spriteType(_v: number | Var) {}
  public get player(): number { return 0; }
  public set player(_v: number | Var) {}
  public get posX(): number { return 0; }
  public set posX(_v: number | Var) {}
  public get posY(): number { return 0; }
  public set posY(_v: number | Var) {}
}

/**
 * Typed handle to a StarCraft: Remastered CUnit structure.
 *
 * `epd` locates the structure in EPD space; `ptr` optionally stores the raw
 * pointer. `CUnit.fromPtr` derives the EPD from a raw pointer.
 */
export class CUnit {
  public ptr: number | Var;
  public epd: number | Var;

  constructor(epd: number | Var, ptr: number | Var = 0) {
    this.epd = epd;
    this.ptr = ptr;
  }

  public static cast(epd: number | Var, ptr: number | Var = 0): CUnit {
    return new CUnit(epd, ptr);
  }

  public static fromPtr(ptr: number | Var): CUnit {
    if (typeof ptr === "number") {
      return new CUnit(EPD(ptr), ptr);
    }
    return new CUnit(ptr, ptr);
  }

  // Common CUnit offsets (relative to EPD):
  // +0x008: hitPoints (dword, 1/256 HP) -> EPD + 2
  // +0x00C: sprite (CSprite*) -> EPD + 3
  // +0x028: posX (word) -> EPD + 10, bits 0..15
  // +0x02A: posY (word) -> EPD + 10, bits 16..31
  // +0x04C: player (byte) -> EPD + 19, bits 0..7
  // +0x04D: order (byte) -> EPD + 19, bits 8..15
  // +0x05C: orderTarget (CUnit*) -> EPD + 23
  // +0x060: shields (dword, 1/256 Shields) -> EPD + 24
  // +0x064: unitType (word) -> EPD + 25, bits 0..15
  // +0x0A2: energy (word) -> EPD + 40, bits 16..31
  // +0x0DC: statusFlags (dword) -> EPD + 55

  public get hitPoints(): number { return 0; }
  public set hitPoints(_v: number | Var) {}

  public get shields(): number { return 0; }
  public set shields(_v: number | Var) {}

  public get energy(): number { return 0; }
  public set energy(_v: number | Var) {}

  public get unitType(): number { return 0; }
  public set unitType(_v: number | Var) {}

  public get player(): number { return 0; }
  public set player(_v: number | Var) {}

  public get posX(): number { return 0; }
  public set posX(_v: number | Var) {}

  public get posY(): number { return 0; }
  public set posY(_v: number | Var) {}

  public get order(): number { return 0; }
  public set order(_v: number | Var) {}

  public get orderTarget(): CUnit | null { return null; }
  public set orderTarget(_v: CUnit | null) {}

  public get sprite(): CSprite | null { return null; }
  public set sprite(_v: CSprite | null) {}
}

/**
 * Fixed-length numeric array for use in compiled scripts.
 *
 * `length` is the number of slots and `initVal` the initial value of each.
 */
export class EUDArray {
  private readonly data: number[];

  constructor(public readonly length: number, initVal: number = 0) {
    this.data = new Array(length).fill(initVal);
  }

  public get(index: number | Var): number {
    const idx = typeof index === "number" ? index : index.initVal;
    return this.data[idx] ?? 0;
  }

  public set(index: number | Var, value: number | Var): void {
    const idx = typeof index === "number" ? index : index.initVal;
    const val = typeof value === "number" ? value : value.initVal;
    this.data[idx] = val;
  }
}
export { EUDArray as Array };

/**
 * Immutable static string registered in the map's STR section.
 *
 * Map strings cannot be formatted; use {@link string} for dynamic text.
 */
export class MapString {
  public readonly id: number = 0;
  public readonly offset: number = 0;
  public readonly address: number = 0x191943C8;
  public readonly epd: number = Math.floor((0x191943C8 - DEATHS_BASE_ADDR) / 4);

  constructor(public readonly text: string) {}

  public display(alwaysDisplay: boolean = true): Action | Action[] {
    return new Action("DisplayTextId", [this.id, alwaysDisplay]);
  }

  public display_at(line: number): Action {
    return display_at(line, this.text);
  }

  public displayAt(line: number): Action {
    return this.display_at(line);
  }

  public format(..._args: any[]): Action {
    throw new TypeError(
      "MapString is static and cannot be formatted. Use string() for dynamic formatted buffers."
    );
  }
}

/**
 * Mutable, format-capable string buffer backed by the map's STRx section.
 *
 * `content` may be an initial string or a byte length to allocate as spaces.
 */
export class StringBuffer extends MapString {
  constructor(content: number | string = 1024) {
    const initialText = typeof content === "number" ? " ".repeat(content) : content;
    super(initialText);
  }

  public printf(...args: any[]): Action {
    return new Action("FormatString", [this.id, ...args]);
  }

  public format(...args: any[]): Action {
    return new Action("FormatString", [this.id, ...args]);
  }

  public display(...args: any[]): Action | Action[] {
    if (args.length > 0 && typeof args[0] !== "boolean") {
      const displayAction = super.display(true);
      return [
        this.format(...args),
        ...(Array.isArray(displayAction) ? displayAction : [displayAction]),
      ];
    }
    const alwaysDisplay = args.length > 0 && typeof args[0] === "boolean" ? args[0] : true;
    return super.display(alwaysDisplay);
  }

  public display_at(line: number, ...args: any[]): Action {
    if (args.length > 0 && typeof args[0] === "string") {
      return display_at(line, args[0], ...args.slice(1));
    }
    return display_at(line, this.text, ...args);
  }

  public displayAt(line: number, ...args: any[]): Action {
    return this.display_at(line, ...args);
  }

  /**
   * @deprecated Use `display_at` or `displayAt` instead.
   */
  public print_at(line: number, ...args: any[]): Action {
    return this.display_at(line, ...args);
  }
}

/**
 * Creates or registers a mutable formatted string buffer in the map's STRx section.
 */
export function string(content: number | string = 1024): StringBuffer {
  return new StringBuffer(content);
}

/**
 * Creates or registers an immutable static string in the map's STR section.
 */
export function map_string(text: string): MapString {
  return new MapString(text);
}

/**
 * Ambient compilation session proxy for TypeScript scripts.
 */
export const session = {
  var: var_,
  string: string,
  string_buffer: string,
  stringBuffer: string,
  map_string: map_string,
  mapString: map_string,
  display_at: display_at,
  displayAt: displayAt,
  DisplayTextAt: DisplayTextAt,
  displayText: DisplayText,
  DisplayText: DisplayText,
};

/**
 * Execute a block of logic exactly once during the game.
 *
 * This is a compile-time primitive: `eudkit-ts` lowers the call to `HirStmt::Reactive`.
 * The HIR reactive lowering module owns the guard variable and its Trigger Loop timing.
 * There is intentionally no runtime implementation in this SDK.
 *
 * If `condition` is provided, waits until the condition holds on a Trigger Loop pass,
 * executes once, and permanently deactivates its guard.
 * If `condition` is omitted, null, or undefined, executes on the initial Trigger Loop pass.
 */
export declare function once(callback: () => void): void;
export declare function once(
  condition: Condition | Condition[] | null | undefined,
  callback: () => void
): void;

/**
 * Standing reactive rule: executes callback every Trigger Loop while condition evaluates to true.
 *
 * `eudkit-ts` lowers the call to `HirStmt::Reactive`; the HIR reactive lowering module
 * owns the condition and body placement. There is intentionally no runtime implementation.
 */
export declare function when(
  condition: Condition | Condition[],
  callback: () => void
): void;

/**
 * Continuous rule: executes callback every Trigger Loop until condition holds, then permanently deactivates.
 *
 * `eudkit-ts` lowers the call to `HirStmt::Reactive`; the HIR reactive lowering module
 * owns the guard variable. There is intentionally no runtime implementation.
 */
export declare function until(
  condition: Condition,
  callback: () => void
): void;

/**
 * Delay rule: counts down `loops` Trigger Loops and fires callback once.
 *
 * `eudkit-ts` lowers the call to `HirStmt::Reactive`; the HIR reactive lowering module
 * owns the timer and completion guard. There is intentionally no runtime implementation.
 *
 * Native `Wait(ms)` actions freeze the player's trigger evaluation thread and cause
 * multiplayer desyncs. `after(loops, cb)` counts Trigger Loops instead.
 *
 * @param loops - Number of Trigger Loop evaluation cycles to wait before firing.
 */
export declare function after(loops: number, callback: () => void): void;

/**
 * Periodic clock rule: fires repeatedly every `loops` Trigger Loops.
 *
 * `eudkit-ts` lowers the call to `HirStmt::Reactive`; the HIR reactive lowering module
 * owns the timer. There is intentionally no runtime implementation.
 *
 * @param loops - Interval in Trigger Loop cycles between successive firings.
 */
export declare function every(loops: number, callback: () => void): void;
