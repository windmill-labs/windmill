import { expect, test } from "bun:test";
import { defaultPartitionValue } from "../src/commands/pipeline/pipeline.ts";

// Mirrors the backend defaults in partition_ee.rs::default_format — a --local
// run resolves the partition client-side, so a divergence here writes a slice
// the deployed cadence never reruns.
test("defaultPartitionValue: time kinds match backend default formats (UTC)", () => {
  const at = new Date(Date.UTC(2026, 6, 2, 6, 30)); // 2026-07-02T06:30Z, a Thursday
  expect(defaultPartitionValue("daily", at)).toBe("2026-07-02");
  expect(defaultPartitionValue("hourly", at)).toBe("2026-07-02T06");
  expect(defaultPartitionValue("weekly", at)).toBe("2026-W27");
  expect(defaultPartitionValue("monthly", at)).toBe("2026-07");
  expect(defaultPartitionValue("dynamic", at)).toBeUndefined();
});

test("defaultPartitionValue: ISO week-year boundaries (%G-W%V)", () => {
  // Jan 1 2026 is a Thursday → week 1 of 2026.
  expect(defaultPartitionValue("weekly", new Date(Date.UTC(2026, 0, 1)))).toBe("2026-W01");
  // Jan 1 2023 is a Sunday → still week 52 of 2022.
  expect(defaultPartitionValue("weekly", new Date(Date.UTC(2023, 0, 1)))).toBe("2022-W52");
  // Dec 30 2024 is a Monday → already week 1 of 2025.
  expect(defaultPartitionValue("weekly", new Date(Date.UTC(2024, 11, 30)))).toBe("2025-W01");
});
