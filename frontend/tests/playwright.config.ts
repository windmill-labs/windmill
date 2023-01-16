// playwright.config.ts
import type { PlaywrightTestConfig } from '@playwright/test'
const config: PlaywrightTestConfig = {
    globalSetup: './global-setup',
    use: {
			// Tell all tests to load signed-in state from 'storageState.json'.
			storageState: 'storageState.json'
    }
}
export default config
