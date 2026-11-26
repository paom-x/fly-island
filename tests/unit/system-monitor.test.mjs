import assert from "node:assert/strict";
import test from "node:test";
import { formatMetric, formatRate, METRIC_SAMPLE_VALUE } from "../../src/features/system-monitor/format.ts";
import { availableMetrics, SYSTEM_METRICS } from "../../src/features/system-monitor/types.ts";

/** A reading with every counter present, so a test states only the field it cares about. */
const STATS = {
  cpuPercent: 37.4,
  memoryPercent: 62.6,
  memoryUsed: 12_000,
  memoryTotal: 19_000,
  networkDown: 1536,
  networkUp: 512,
  gpuPercent: 12.2,
  temperature: 41.6,
};

test("a byte rate keeps one decimal below ten so a quiet connection still moves", () => {
  assert.equal(formatRate(0), "0 B/s");
  assert.equal(formatRate(512), "512 B/s");
  assert.equal(formatRate(1536), "1.5 KB/s");
  assert.equal(formatRate(10 * 1024), "10 KB/s");
  assert.equal(formatRate(2.5 * 1024 * 1024), "2.5 MB/s");
  assert.equal(formatRate(1024 ** 3), "1 GB/s");
  // A counter that reports nonsense must not print NaN into the bar.
  assert.equal(formatRate(Number.NaN), "0 B/s");
  assert.equal(formatRate(-4), "0 B/s");
});

test("a metric reads as its own unit, down before up for the network", () => {
  assert.equal(formatMetric(STATS, "cpu"), "37%");
  assert.equal(formatMetric(STATS, "memory"), "63%");
  assert.equal(formatMetric(STATS, "network"), "↓1.5 KB/s ↑512 B/s");
  assert.equal(formatMetric(STATS, "gpu"), "12%");
  assert.equal(formatMetric(STATS, "temperature"), "42°C");
});

test("a machine without the counter prints nothing for it rather than a zero", () => {
  const bare = { ...STATS, gpuPercent: null, temperature: null };
  assert.equal(formatMetric(bare, "gpu"), "");
  assert.equal(formatMetric(bare, "temperature"), "");
});

test("only the counters this machine can read are offered", () => {
  assert.deepEqual(availableMetrics(STATS, SYSTEM_METRICS), [...SYSTEM_METRICS]);
  const bare = { ...STATS, gpuPercent: null, temperature: null };
  assert.deepEqual(availableMetrics(bare, SYSTEM_METRICS), ["cpu", "memory", "network"]);
  // What the user ticked is kept, in their order, once the missing ones are gone.
  assert.deepEqual(availableMetrics(bare, ["temperature", "network", "cpu"]), ["network", "cpu"]);
});

test("every counter has a sample reading wide enough to size the bar once for all of them", () => {
  for (const metric of SYSTEM_METRICS) {
    assert.ok(METRIC_SAMPLE_VALUE[metric].length >= 4, `${metric} needs a sample wide enough to size the bar`);
  }
});
