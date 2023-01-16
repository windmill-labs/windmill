// playwright.config.ts
import type { PlaywrightTestConfig } from '@playwright/test'
const config: PlaywrightTestConfig = {
    globalSetup: './global-setup',
    use: {
			// Tell all tests to load signed-in state from 'storageState.json'.
			storageState: 'storageState.json',
			// baseURL is set in the globalSetup script as well, because it's not affected by this config.
			// Make sure to update it there as well when this is updated.
			baseURL: process.env.BASE_URL || 'http://localhost'
    }
}
export default config
