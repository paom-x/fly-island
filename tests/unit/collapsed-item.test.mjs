import assert from "node:assert/strict";
import test from "node:test";
import { pickCollapsedItem } from "../../src/windows/island/collapsedItem.ts";

const TODO_FIRST = ["todo", "agent", "lyrics"];
const DEFAULT_ORDER = ["agent", "lyrics", "todo"];

test("a started todo ranked above the agent is what the collapsed island shows", () => {
  assert.equal(pickCollapsedItem(TODO_FIRST, { todo: true, agent: true, lyrics: true, system: false }, true), "todo");
  // Ranked below, it still waits its turn.
  assert.equal(pickCollapsedItem(DEFAULT_ORDER, { todo: true, agent: true, lyrics: false, system: false }, true), "agent");
});

test("with no todo started, the remaining count gives way to a running agent or a lyric line", () => {
  assert.equal(pickCollapsedItem(TODO_FIRST, { todo: true, agent: true, lyrics: false, system: false }, false), "agent");
  assert.equal(pickCollapsedItem(TODO_FIRST, { todo: true, agent: false, lyrics: true, system: false }, false), "lyrics");
  assert.equal(pickCollapsedItem(["todo", "lyrics", "agent"], { todo: true, agent: true, lyrics: true, system: false }, false), "lyrics");
});

test("an unstarted todo still shows when nothing else has anything", () => {
  assert.equal(pickCollapsedItem(TODO_FIRST, { todo: true, agent: false, lyrics: false, system: false }, false), "todo");
  assert.equal(pickCollapsedItem(DEFAULT_ORDER, { todo: true, agent: false, lyrics: false, system: false }, false), "todo");
});

test("nothing to show leaves the text place empty", () => {
  assert.equal(pickCollapsedItem(TODO_FIRST, { todo: false, agent: false, lyrics: false, system: false }, false), null);
  assert.equal(pickCollapsedItem(DEFAULT_ORDER, { todo: false, agent: false, lyrics: false, system: false }), null);
});

test("ticked machine counters rank like the rest, by the order the user set", () => {
  const withCounters = ["system", "agent", "lyrics", "todo"];
  assert.equal(pickCollapsedItem(withCounters, { todo: false, agent: false, lyrics: false, system: true }), "system");
  // Rank is the order the user set: with the agent first, it wins even though the counters
  // always have a reading of their own.
  assert.equal(pickCollapsedItem(["agent", "system"], { todo: false, agent: true, lyrics: false, system: true }), "agent");
  // Ticketed nowhere, the counters claim nothing.
  assert.equal(pickCollapsedItem(["system", "lyrics"], { todo: false, agent: false, lyrics: true, system: false }), "lyrics");
});
