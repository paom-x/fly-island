import test from "node:test";
import assert from "node:assert/strict";

import { DEFAULT_SETTINGS, normalizeSettings } from "../../src/features/appearance/settings.ts";

// The burn-in guard steps the collapsed island along the top edge; it is on unless a save says
// otherwise, and a save that never had the switch keeps it on.
test("the burn-in guard is on by default and stays off once saved off", () => {
  assert.equal(DEFAULT_SETTINGS.oledShift, true);
  assert.equal(normalizeSettings(null).oledShift, true);
  assert.equal(normalizeSettings({}).oledShift, true);
  assert.equal(normalizeSettings({ oledShift: false }).oledShift, false);
  assert.equal(normalizeSettings({ oledShift: "no" }).oledShift, true);
});
