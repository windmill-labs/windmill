import { test, expect } from '@playwright/test'

test('has Home text', async ({ page }) => {
	await page.goto('/')
	await expect(page.locator('text=Home').first()).toBeVisible()
})
