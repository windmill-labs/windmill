import { test, expect } from '@playwright/test'

test('log in correctly', async ({ page }) => {
	await page.goto('/')

	// Wait for and fill the email input
	const emailInput = page.locator('input#email[type="email"]')
	await emailInput.waitFor({ state: 'visible' })
	await emailInput.fill('admin@windmill.dev')

	// Wait for and fill the password input
	const passwordInput = page.locator('input#password[type="password"]')
	await passwordInput.waitFor({ state: 'visible' })
	await passwordInput.fill('changeme')

	// Verify the inputs are filled correctly
	await expect(emailInput).toHaveValue('admin@windmill.dev')
	await expect(passwordInput).toHaveValue('changeme')

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
	await expect(page).toHaveURL(/\/user\/workspaces$/)

	// Click on the "Admins" workspace
	const adminsWorkspace = page.locator('text=Admins').first()
	await adminsWorkspace.waitFor({ state: 'visible' })
	await adminsWorkspace.click()

	// Verify we're on the root path of the workspace
	await page.waitForURL('**/')
	await expect(page).toHaveURL(/\/$/)

	// Wait for the page to load and verify it contains "Home"
	await expect(page.locator('text=Home').first()).toBeVisible()
})
