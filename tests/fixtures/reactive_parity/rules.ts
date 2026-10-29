/**
 * Reactive Rule fixture shared by the Python/TypeScript parity test.
 */

import {
  Add,
  DisplayText,
  Ore,
  Player1,
  SetResources,
  after,
  every,
  once,
  session,
  until,
  when,
} from "eudkit";

export const kills = session.var(0, "kills");
export const timer = session.var(10, "timer");

export function main() {
  once(() => {
    DisplayText("init");
  });
  once(kills.get() >= 5, () => {
    DisplayText("milestone");
  });
  when([kills.get() >= 10, timer.get() == 0], () => {
    DisplayText("standing");
  });
  until(timer.get() == 0, () => {
    DisplayText("countdown");
  });
  after(24, () => {
    DisplayText("delayed");
  });
  every(12, () => {
    DisplayText("periodic");
  });
  SetResources(Player1, Add, 50, Ore);
}
