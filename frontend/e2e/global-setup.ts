// global-setup.ts
import { chromium, FullConfig } from '@playwright/test'

async function globalSetup(config: FullConfig) {
	process.env.TEST_UNIQUE_ID = Date.now().toString()

	const browser = await chromium.launch()
	const context = await browser.newContext({
		permissions: ['clipboard-read', 'clipboard-write']
	})
	const page = await context.newPage()

	// Use baseURL from config
	const baseURL = config.projects[0].use.baseURL || 'http://localhost:3000'
	await page.goto(baseURL)

	// Wait for and fill the email input
	const emailInput = page.locator('input#email[type="email"]')
	await emailInput.waitFor({ state: 'visible' })
	await emailInput.fill('admin@windmill.dev')

	// Wait for and fill the password input
	const passwordInput = page.locator('input#password[type="password"]')
	await passwordInput.waitFor({ state: 'visible' })
	await passwordInput.fill('changeme')

	// Click the sign-in button
	const signInButton = page.locator('button:has-text("Sign in")')
	await signInButton.click()

	// Handle potential first-time user flow
	await page.waitForURL(/\/user\/(first-time|workspaces)/)
	if (page.url().includes('/user/first-time')) {
		const skipButton = page.locator('button:has-text("Skip")')
		await skipButton.click()
	}

	// Wait for navigation to workspaces page
	await page.waitForURL('**/user/workspaces')

	// Click on the "Admins" workspace
	const adminsWorkspace = page.locator('text=Admins').first()
	await adminsWorkspace.waitFor({ state: 'visible' })
	await adminsWorkspace.click()

	// Wait for workspace to load
	await page.waitForURL('**/')

	if (!process.env.SKIP_HUB_SYNC) {
		await page.locator('#home-search-input').waitFor({ state: 'visible' })
		await page.locator('#home-search-input').fill('u/admin/hub_sync')

		const hubSyncBtn = page.locator('text=u/admin/hub_sync').first()
		await hubSyncBtn.waitFor({ state: 'visible' })
		await hubSyncBtn.click()

		const runBtn = page.locator('#run-form-run-button')
		await runBtn.waitFor({ state: 'visible' })
		await runBtn.click()
		await page.waitForURL(/\/run\/.+/)
		await page.locator('text=Success').waitFor({ timeout: 30000, state: 'visible' })
	}
	// Save the authenticated state
	await context.storageState({ path: './e2e/auth.json' })
	await browser.close()
}

export default globalSetup

declare const process: any // ignore TS errors
