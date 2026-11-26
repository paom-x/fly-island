import assert from "node:assert/strict";
import test from "node:test";
import { normalizeSettings } from "../../src/features/appearance/settings.ts";

// Settings -> Messages -> "How much to read": the mode reaches Rust
// (src-tauri/src/message_reading.rs), which only understands the two names, so an
// unknown value has to fall back to reading the text rather than silently going quiet.
test("notifications read their content by default", () => {
  assert.equal(normalizeSettings(null).messageReading, "content");
  assert.equal(normalizeSettings({}).messageReading, "content");
});

test("only-unread survives a save, an unknown mode falls back to content", () => {
  assert.equal(normalizeSettings({ messageReading: "unread" }).messageReading, "unread");
  assert.equal(normalizeSettings({ messageReading: "content" }).messageReading, "content");
  assert.equal(normalizeSettings({ messageReading: "summary" }).messageReading, "content");
  assert.equal(normalizeSettings({ messageReading: 0 }).messageReading, "content");
});
