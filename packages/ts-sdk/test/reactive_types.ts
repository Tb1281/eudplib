import {
  Add,
  DisplayText,
  Ore,
  Player1,
  SetResources,
  after,
  every,
  once,
  until,
  var_,
  when,
} from "../src/index.js";

// @ts-expect-error misspelled reactive primitives must not be exported
import { oncee } from "../src/index.js";

const flag = var_(0, "flag");
const other = var_(0, "other");

function callback(): void {
  DisplayText("callback");
}

// Valid overloads: once(callback) and once(condition, callback).
once(callback);
once(() => DisplayText("arrow callback"));
once(flag.atLeast(1), callback);
once([flag.atLeast(1), other.atMost(2)], callback);
once(null, callback);
once(undefined, callback);
once(flag.atLeast(1), () => {
  DisplayText("condition with action body");
  SetResources(Player1, Add, 50, Ore);
});

// Valid condition and loop shapes.
when(flag.atLeast(1), callback);
when([flag.atLeast(1), other.atMost(2)], callback);
until(flag.exactly(0), callback);
after(24, callback);
every(12, callback);

// Invalid shapes. These must be type errors, not silent no-ops.
// @ts-expect-error once requires a callback
once();
// @ts-expect-error once(condition) is missing its callback
once(flag.atLeast(1));
// @ts-expect-error once(condition, callback) does not accept a function as the condition
once(callback, callback);
// @ts-expect-error when requires a condition and a callback
when(callback, callback);
// @ts-expect-error when is missing its callback
when(flag.atLeast(1));
// @ts-expect-error until is missing its callback
until(flag.exactly(0));
// @ts-expect-error after requires a numeric loop count
after("24", callback);
// @ts-expect-error every is missing its callback
every(12);
// @ts-expect-error after accepts exactly two arguments
after(24, callback, callback);
