import {
  var_,
  once,
  when,
  until,
  after,
  every,
  DisplayText,
  SetResources,
  Player1,
  Add,
  Ore,
} from "@eudkit/sdk";

let kills = var_(0, "kills");
let timer = var_(10, "timer");
let score = var_(50, "score");

export function main() {
  once(() => {
    DisplayText("once without condition");
    SetResources(Player1, Add, 50, Ore);
  });

  once(score.atLeast(100), () => {
    DisplayText("once with condition");
  });

  when(kills.atLeast(10), () => {
    DisplayText("when condition");
  });

  until(timer.exactly(0), () => {
    DisplayText("until condition");
  });

  after(24, () => {
    DisplayText("after delay");
  });

  every(12, () => {
    DisplayText("every period");
  });
}
