import assert from "node:assert/strict";
import test from "node:test";
import { normalizeSettings } from "../../src/features/appearance/settings.ts";

// Settings → General → Memory: the policy reaches Rust (src-tauri/src/memory_trim.rs), which
// only knows the three names, so anything else has to fall back to the default here.
test("the memory mode defaults to balanced", () => {
  assert.equal(normalizeSettings(null).memoryMode, "balanced");
  assert.equal(normalizeSettings({}).memoryMode, "balanced");
});

test("a saved memory mode survives, an unknown one falls back", () => {
  assert.equal(normalizeSettings({ memoryMode: "saver" }).memoryMode, "saver");
  assert.equal(normalizeSettings({ memoryMode: "smooth" }).memoryMode, "smooth");
  assert.equal(normalizeSettings({ memoryMode: "turbo" }).memoryMode, "balanced");
  assert.equal(normalizeSettings({ memoryMode: 3 }).memoryMode, "balanced");
});
