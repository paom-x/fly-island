// Which screen edge the island is docked to: the default, and what a saved value may say.
import test from "node:test";
import assert from "node:assert/strict";

import { DEFAULT_SETTINGS, normalizeSettings } from "../../src/features/appearance/settings.ts";

test("the island starts docked to the top edge", () => {
  assert.equal(DEFAULT_SETTINGS.dockEdge, "top");
  assert.equal(normalizeSettings(null).dockEdge, "top");
  assert.equal(normalizeSettings({}).dockEdge, "top");
});

test("the sides survive a round trip through the settings file", () => {
  assert.equal(normalizeSettings({ dockEdge: "left" }).dockEdge, "left");
  assert.equal(normalizeSettings({ dockEdge: "right" }).dockEdge, "right");
  assert.equal(normalizeSettings({ dockEdge: "top" }).dockEdge, "top");
});

test("anything else falls back to the top edge", () => {
  // A value written by a newer version, or a hand-edited file, must not leave the island nowhere.
  assert.equal(normalizeSettings({ dockEdge: "bottom" }).dockEdge, "top");
  assert.equal(normalizeSettings({ dockEdge: 2 }).dockEdge, "top");
  assert.equal(normalizeSettings({ dockEdge: null }).dockEdge, "top");
});
