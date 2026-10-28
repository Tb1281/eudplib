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

export class Expr {
  constructor(public readonly raw: any) {}
}

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

export class Condition {
  constructor(public readonly typeName: string, public readonly args: any[]) {}
}

export function Always(): Condition {
  return new Condition("Always", []);
}

export function Never(): Condition {
  return new Condition("Never", []);
}

export function IsUserCP(): Condition {
  return new Condition("IsUserCP", []);
}

export function Deaths(
  player: ExprLike,
  comparison: number,
  amount: ExprLike,
  unit: number | string = 0
): Condition {
  return new Condition("Deaths", [player, comparison, amount, unit]);
}

export function DeathsX(
  player: ExprLike,
  comparison: number,
  amount: ExprLike,
  unit: number | string = 0,
  mask: number = 0xFFFFFFFF
): Condition {
  return new Condition("DeathsX", [player, comparison, amount, unit, mask]);
}

export function Memory(
  address: number,
  comparison: number,
  value: ExprLike
): Condition {
  return new Condition("Memory", [address, comparison, value]);
}

export function MemoryX(
  address: number,
  comparison: number,
  value: ExprLike,
  mask: number
): Condition {
  return new Condition("MemoryX", [address, comparison, value, mask]);
}

export function Accumulate(
  player: number,
  comparison: number,
  amount: number,
  resource: number = Ore
): Condition {
  return new Condition("Accumulate", [player, comparison, amount, resource]);
}

export function Command(
  player: number,
  comparison: number,
  amount: number,
  unit: number | string = 0
): Condition {
  return new Condition("Command", [player, comparison, amount, unit]);
}

export function Bring(
  player: number,
  comparison: number,
  amount: number,
  unit: number | string,
  location: number | string
): Condition {
  return new Condition("Bring", [player, comparison, amount, unit, location]);
}

export function Switch(
  switchId: number | string,
  state: number = Set
): Condition {
  return new Condition("Switch", [switchId, state]);
}

export function ElapsedTime(comparison: number, time: number): Condition {
  return new Condition("ElapsedTime", [comparison, time]);
}

export function Score(
  player: number,
  comparison: number,
  amount: number,
  scoreType: number = 0
): Condition {
  return new Condition("Score", [player, comparison, amount, scoreType]);
}

// --- Actions ---

export class Action {
  constructor(public readonly typeName: string, public readonly args: any[]) {}
}

export function SetDeaths(
  player: ExprLike,
  modifier: number,
  value: ExprLike,
  unit: number | string = 0
): Action {
  return new Action("SetDeaths", [player, modifier, value, unit]);
}

export function SetDeathsX(
  player: ExprLike,
  modifier: number,
  value: ExprLike,
  unit: number | string = 0,
  mask: number = 0xFFFFFFFF
): Action {
  return new Action("SetDeathsX", [player, modifier, value, unit, mask]);
}

export function SetMemory(
  address: number,
  modifier: number,
  value: ExprLike
): Action {
  return new Action("SetMemory", [address, modifier, value]);
}

export function SetMemoryX(
  address: number,
  modifier: number,
  value: ExprLike,
  mask: number
): Action {
  return new Action("SetMemoryX", [address, modifier, value, mask]);
}

export function SetResources(
  player: number,
  modifier: number,
  amount: number,
  resource: number = Ore
): Action {
  return new Action("SetResources", [player, modifier, amount, resource]);
}

export function DisplayText(text: string, alwaysDisplay: boolean = true): Action {
  return new Action("DisplayText", [text, alwaysDisplay]);
}

export function displayText(template: string, ...args: ExprLike[]): Action {
  return new Action("displayText", [template, ...args]);
}

export function f_eprintln(template: string, ...args: ExprLike[]): Action {
  return new Action("f_eprintln", [template, ...args]);
}

export function display_at(line: number, template: string, ...args: ExprLike[]): Action {
  return new Action("display_at", [line, template, ...args]);
}

export const displayAt = display_at;
export const DisplayTextAt = display_at;

/**
 * @deprecated Use `display_at` or `displayAt` instead.
 */
export const print_at = display_at;

export function DisplayTextId(stringId: number, alwaysDisplay: boolean = true): Action {
  return new Action("DisplayTextId", [stringId, alwaysDisplay]);
}

export function CreateUnit(
  count: number,
  unit: number | string,
  location: number | string,
  player: number
): Action {
  return new Action("CreateUnit", [count, unit, location, player]);
}

export function Comment(text: string): Action {
  return new Action("Comment", [text]);
}

export function PreserveTrigger(): Action {
  return new Action("PreserveTrigger", []);
}

export function Victory(): Action {
  return new Action("Victory", []);
}

export function Defeat(): Action {
  return new Action("Defeat", []);
}

export function KillUnit(unit: number | string, player: number): Action {
  return new Action("KillUnit", [unit, player]);
}

export function KillUnitAt(
  count: number,
  unit: number | string,
  location: number | string,
  player: number
): Action {
  return new Action("KillUnitAt", [count, unit, location, player]);
}

export function RemoveUnitAt(
  count: number,
  unit: number | string,
  location: number | string,
  player: number
): Action {
  return new Action("RemoveUnitAt", [count, unit, location, player]);
}

export function MoveLocation(
  location: number | string,
  onUnit: number | string,
  owner: number,
  destLocation: number | string
): Action {
  return new Action("MoveLocation", [location, onUnit, owner, destLocation]);
}

export function MoveUnit(
  count: number,
  unit: number | string,
  player: number,
  srcLocation: number | string,
  destLocation: number | string
): Action {
  return new Action("MoveUnit", [count, unit, player, srcLocation, destLocation]);
}

export function GiveUnits(
  count: number,
  unit: number | string,
  player: number,
  srcLocation: number | string,
  toPlayer: number
): Action {
  return new Action("GiveUnits", [count, unit, player, srcLocation, toPlayer]);
}

export function Order(
  unit: number | string,
  player: number,
  srcLocation: number | string,
  orderType: number,
  destLocation: number | string
): Action {
  return new Action("Order", [unit, player, srcLocation, orderType, destLocation]);
}

export function PlayWAV(wavFile: string): Action {
  return new Action("PlayWAV", [wavFile]);
}

export function MinimapPing(location: number | string): Action {
  return new Action("MinimapPing", [location]);
}

export function SetScore(
  player: number,
  modifier: number,
  amount: number,
  scoreType: number = 0
): Action {
  return new Action("SetScore", [player, modifier, amount, scoreType]);
}

export function SetSwitch(switchId: number | string, action: number = Set): Action {
  return new Action("SetSwitch", [switchId, action]);
}

// --- Memory Read/Write Operations ---

export function dwread(ptr: number | Var): number { return 0; }
export function dwread_epd(epd: number | Var): number { return 0; }
export function epdread_epd(epd: number | Var): number { return 0; }
export function bread(ptr: number | Var): number { return 0; }
export function bread_epd(epd: number | Var): number { return 0; }
export function wread(ptr: number | Var): number { return 0; }
export function wread_epd(epd: number | Var): number { return 0; }

export function dwwrite(ptr: number | Var, val: number | Var): Action {
  return SetMemory(typeof ptr === "number" ? ptr : 0, SetTo, val);
}
export function dwwrite_epd(epd: number | Var, val: number | Var): Action {
  return SetDeaths(epd, SetTo, val, 0);
}
export function bwrite(ptr: number | Var, val: number | Var): Action {
  return SetMemory(typeof ptr === "number" ? ptr : 0, SetTo, val);
}
export function bwrite_epd(epd: number | Var, val: number | Var): Action {
  return SetDeaths(epd, SetTo, val, 0);
}
export function wwrite(ptr: number | Var, val: number | Var): Action {
  return SetMemory(typeof ptr === "number" ? ptr : 0, SetTo, val);
}
export function wwrite_epd(epd: number | Var, val: number | Var): Action {
  return SetDeaths(epd, SetTo, val, 0);
}

export function DoActions(...actions: Action[]): void {
  // Runtime stub; AST lowering compiles action calls directly
}

// --- Control Flow & Metaprogramming ---

export function loopRange(start: number, stop?: number, step: number = 1): number[] {
  const actualStart = stop === undefined ? 0 : start;
  const actualStop = stop === undefined ? start : stop;
  const result: number[] = [];
  for (let i = actualStart; step > 0 ? i < actualStop : i > actualStop; i += step) {
    result.push(i);
  }
  return result;
}

export interface FnOptions {
  mainLoop?: boolean;
  inline?: "auto" | "always" | "never";
}

export function fn(options?: FnOptions): (target: any) => any;
export function fn(target: any): any;
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

export class MapString {
  public readonly id: number = 0;
  public readonly offset: number = 0;
  public readonly address: number = 0x191943C8;
  public readonly epd: number = Math.floor((0x191943C8 - DEATHS_BASE_ADDR) / 4);

  constructor(public readonly text: string) {}

  public display(alwaysDisplay: boolean = true): Action {
    return new Action("DisplayTextId", [this.id, alwaysDisplay]);
  }

  public display_at(line: number): Action {
    return display_at(line, this.text);
  }

  public displayAt(line: number): Action {
    return this.display_at(line);
  }

  public format(..._args: any[]): never {
    throw new TypeError(
      "MapString is static and cannot be formatted. Use string() for dynamic formatted buffers."
    );
  }
}

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
      return [this.format(...args), super.display(true)];
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
  string_buffer: stringBuffer,
  stringBuffer: stringBuffer,
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
 * If `condition` is provided, waits until the condition holds on a trigger loop pass,
 * executes once, and permanently deactivates its guard.
 * If `condition` is omitted or null, executes on the initial trigger loop pass.
 *
 * @param conditionOrCallback - Condition guard or callback function.
 * @param maybeCallback - Callback function if condition was provided as first argument.
 */
export function once(
  conditionOrCallback: Condition | null | undefined | (() => void),
  maybeCallback?: () => void
): void {
  const callback =
    typeof conditionOrCallback === "function" ? conditionOrCallback : maybeCallback;
  const condition =
    typeof conditionOrCallback === "function" ? null : conditionOrCallback;

  const flag = var_(1);
  if (condition) {
    if (flag.exactly(1) && condition) {
      flag.setTo(0);
      callback?.();
    }
  } else {
    if (flag.exactly(1)) {
      flag.setTo(0);
      callback?.();
    }
  }
}

/**
 * Standing reactive rule: executes callback every trigger loop while condition evaluates to true.
 *
 * In StarCraft's polling-based trigger VM, `when(cond, cb)` checks conditions at the beginning
 * of each trigger loop pass. As long as the conditions remain satisfied, the enclosed actions execute.
 *
 * @param condition - Single condition or array of conditions to evaluate every loop.
 * @param callback - Function body containing actions to execute while conditions hold.
 */
export function when(condition: Condition | Condition[], callback: () => void): void {
  const isSatisfied = Array.isArray(condition)
    ? condition.length > 0 && condition.every((c) => Boolean(c))
    : Boolean(condition);
  if (isSatisfied) {
    callback();
  }
}

/**
 * Continuous rule: executes callback every trigger loop until condition holds, then permanently deactivates.
 *
 * Evaluated on each trigger loop pass. As soon as `condition` becomes true, the guard flag is set
 * to 1 and the block will never execute again in subsequent trigger loops.
 *
 * @param condition - Termination condition that permanently silences this rule.
 * @param callback - Function body executed each trigger loop prior to termination.
 */
export function until(condition: Condition, callback: () => void): void {
  const guard = var_(0);
  if (guard.exactly(0)) {
    if (condition) {
      guard.setTo(1);
    } else {
      callback();
    }
  }
}

/**
 * Delay rule: counts down `loops` trigger cycles and fires callback once.
 *
 * ### StarCraft Trigger Loop Architecture
 * In Brood War, triggers do NOT execute on every engine frame by default; they run in discrete
 * Trigger Loops (roughly every 30-31 game frames in standard maps, or every single frame when
 * an EUD Turbo Trigger / Hyper Trigger continuously resets the trigger loop tick countdown to 0).
 *
 * Native `Wait(ms)` actions freeze the player's trigger evaluation thread and cause multiplayer desyncs.
 * `after(loops, cb)` avoids `Wait` entirely by maintaining a deterministic 4-byte counter variable
 * decremented on each trigger loop pass without thread stalls.
 *
 * @param loops - Number of trigger loop evaluation cycles to wait before firing (e.g. 24 loops = ~1 second under Turbo).
 * @param callback - Logic to execute once the loop delay has elapsed.
 */
export function after(loops: number, callback: () => void): void {
  const timer = var_(loops);
  const done = var_(0);
  if (done.exactly(0)) {
    if (timer.atLeast(1)) {
      timer.subtract(1);
    }
    if (timer.exactly(0)) {
      done.setTo(1);
      callback();
    }
  }
}

/**
 * Periodic clock rule: fires repeatedly every `loops` trigger cycles.
 *
 * ### StarCraft Trigger Loop Architecture
 * In Brood War, triggers execute in discrete Trigger Loops. In standard maps, 1 loop occurs
 * every ~31 frames (~1.3s on Fastest speed). When an EUD Turbo Trigger is active, the engine's
 * trigger loop countdown tick is kept at 0, causing the trigger loop to execute on every single
 * game frame (24 times/second on Fastest).
 *
 * `every(loops, cb)` maintains a deterministic countdown variable that decrements on each trigger
 * loop cycle and resets to `loops` upon reaching 0, executing without relying on fragile engine `Wait()` calls.
 *
 * @param loops - Interval in trigger loop cycles between successive firings (e.g. 24 loops = every 1s under Turbo).
 * @param callback - Logic to execute periodically every `loops` trigger cycles.
 */
export function every(loops: number, callback: () => void): void {
  const timer = var_(loops);
  if (timer.atLeast(1)) {
    timer.subtract(1);
  }
  if (timer.exactly(0)) {
    timer.setTo(loops);
    callback();
  }
}



