import { getRootStore } from "../store.ts";

/**
 * Create a temporary config directory for testing that doesn't interfere with user's config
 */
export async function withTestConfig<T>(callback: (testConfigDir: string) => Promise<T>): Promise<T> {
  // Create a unique temporary directory for this test
  const testDir = await Deno.makeTempDir({ prefix: "wmill_test_config_" });
  
  try {
    return await callback(testDir);
  } finally {
    // Clean up the temporary directory
    try {
      await Deno.remove(testDir, { recursive: true });
    } catch (error) {
      console.warn(`Failed to clean up test config directory ${testDir}:`, error);
    }
  }
}

/**
 * Clear the remotes file in test config directory
 */
export async function clearTestRemotes(testConfigDir: string): Promise<void> {
  const remoteFile = (await getRootStore(testConfigDir)) + "remotes.ndjson";
  await Deno.writeTextFile(remoteFile, "");
}