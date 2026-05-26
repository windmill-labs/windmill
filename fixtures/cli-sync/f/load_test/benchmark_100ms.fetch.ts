export async function main() {
  const start = Date.now();
  await Bun.sleep(100);
  return { duration_ms: Date.now() - start, expected_ms: 100 };
}