import { chromium, FullConfig } from '@playwright/test'

async function globalSetup(config: FullConfig) {
    const browser = await chromium.launch()
    const page = await browser.newPage()
    await page.goto('/user/login')
    await page.fill('#email', 'admin@windmill.dev')
    await page.fill('#password', 'changeme')
    await page.click('.flex > .default-button')
    // Save signed-in state to 'storageState.json'.
    await page.context().storageState({ path: 'storageState.json' })
    await browser.close()
}

export default globalSetup
