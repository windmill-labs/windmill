import { chromium } from '@playwright/test'

async function globalSetup() {
	const browser = await chromium.launch()
	const page = await browser.newPage({baseURL: 'http://localhost:8000' })
	await page.goto('/user/login', { waitUntil: 'networkidle' })

	if (await page.locator('#email').isHidden()) {
		await page.locator('button', {
			hasText: 'Login without third-party'
		}).click()
	}

	const email = page.locator('input[type="email"]')
	await email.fill('user@windmill.dev')

	const password = page.locator('input[type="password"]')
	await password.fill('changeme')

	await page.locator('#login2').click()
	await page.waitForResponse('/api/auth/login')
	await page.context().storageState({ path: 'storageState.json' })
	await browser.close()
}

export default globalSetup
