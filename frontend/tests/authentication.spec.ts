import { test, expect } from '@playwright/test'


test('can login', async ({ page }) => {
    await page.goto('http://localhost:8000/')
    await expect(page.locator('text=Select a workspace')).toBeVisible()
})
