import { chromium, FullConfig } from '@playwright/test'

async function globalSetup(config: FullConfig) {
	console.log('start global setup!')
	const browser = await chromium.launch()
	const page = await browser.newPage()
	await page.goto('http://localhost:8000/user/login')

	if (!(await page.locator('#email').isVisible())) {
		page.locator('text=login without third-party').click()
	}

	await page.locator('#email').fill('admin@windmill.dev')
	await page.locator('input[type="password"]').fill('changeme')
	await page.locator('text="Login"').click()
	await page.waitForResponse('http://localhost:8000/api/auth/login')
	await page.context().storageState({ path: 'storageState.json' })
	await browser.close()
	console.log('end global setup!')
}

export default globalSetup
