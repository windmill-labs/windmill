import { test, expect } from '@playwright/test'

test('can visit workspace selector page', async ({ page }) => {
	await page.goto('/user/workspaces')
	await expect(page.locator('h1', { hasText: 'Select a workspace' })).toBeVisible()
})
