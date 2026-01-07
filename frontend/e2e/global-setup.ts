// global-setup.ts
import { chromium, FullConfig } from '@playwright/test'

async function globalSetup(config: FullConfig) {
	const browser = await chromium.launch()
	const context = await browser.newContext()
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
	await page.locator('text=Home').first().waitFor({ state: 'visible' })

	const licenseKey = process.env.LICENSE_KEY

	if (licenseKey) {
		// Navigate to superadmin settings to configure license key
		await page.goto(`${baseURL}/#superadmin-settings`)

		// Wait for the license key input
		const licenseKeyInput = page.locator('input#license_key')
		await licenseKeyInput.waitFor({ state: 'visible' })

		// Fill in the license key from environment variable

		await licenseKeyInput.fill(licenseKey)

		// Click the Save button
		const saveButton = page.locator('button:has-text("Save")')
		await saveButton.click()

		// Wait a moment for the save to complete
		await page.waitForTimeout(1000)
	}

	// Save the authenticated state
	await context.storageState({ path: './e2e/auth.json' })
	await browser.close()
}

export default globalSetup
