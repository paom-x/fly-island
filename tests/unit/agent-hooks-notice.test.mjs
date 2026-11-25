import assert from "node:assert/strict";
import test from "node:test";
import { getAgentHooksNotice } from "../../src/features/agent-status/status.ts";

const state = (codex, claudeCode, scriptsOutdated = false, zcode = "current") => ({ codex, claudeCode, zcode, scriptsOutdated });

test("no notice before the state is read or when the hooks are current", () => {
  assert.equal(getAgentHooksNotice(null), null);
  assert.equal(getAgentHooksNotice(state("current", "current")), null);
  // Only one agent installed is fine: Install/Repair would not change anything for it.
  assert.equal(getAgentHooksNotice(state("notInstalled", "current")), null);
});

test("zcode hooks alone never raise or clear the notice", () => {
  // Z Code hooks only feed the island light and Z Code may not be installed, so they gate nothing.
  assert.equal(getAgentHooksNotice(state("current", "current", false, "notInstalled")), null);
  assert.equal(getAgentHooksNotice(state("notInstalled", "notInstalled", false, "current")), "install");
});

test("hooks or scripts from an older version ask for an update", () => {
  assert.equal(getAgentHooksNotice(state("current", "outdated")), "update");
  assert.equal(getAgentHooksNotice(state("outdated", "notInstalled")), "update");
  assert.equal(getAgentHooksNotice(state("current", "current", true)), "update");
});

test("nothing installed asks for an install", () => {
  assert.equal(getAgentHooksNotice(state("notInstalled", "notInstalled")), "install");
});

test("an agent switched off cannot raise the notice", () => {
  // Both sources off: the hooks serve nothing the user can see, so say nothing at all.
  assert.equal(getAgentHooksNotice(state("notInstalled", "notInstalled"), { codex: false, claudeCode: false }), null);
  assert.equal(getAgentHooksNotice(state("outdated", "outdated"), { codex: false, claudeCode: false }), null);
  assert.equal(getAgentHooksNotice(state("current", "current", true), { codex: false, claudeCode: false }), null);
  // One switched off leaves the notice decided by the other.
  assert.equal(getAgentHooksNotice(state("notInstalled", "current"), { codex: false, claudeCode: true }), null);
  assert.equal(getAgentHooksNotice(state("current", "notInstalled"), { codex: false, claudeCode: true }), "install");
  assert.equal(getAgentHooksNotice(state("outdated", "current"), { codex: true, claudeCode: false }), "update");
  assert.equal(getAgentHooksNotice(state("outdated", "current"), { codex: false, claudeCode: true }), null);
  // A source the caller says nothing about still counts, as before.
  assert.equal(getAgentHooksNotice(state("notInstalled", "notInstalled"), { claudeCode: true }), "install");
});
