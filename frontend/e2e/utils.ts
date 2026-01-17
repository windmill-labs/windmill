import { expect, Page } from '@playwright/test'

export class Toast {
	static async expectSuccess(page: Page, message: string) {
		const toast = page.locator(`.toast-success:has-text("${message}")`)
		await expect(toast).toBeVisible({ timeout: 10000 })
	}

	static async expectError(page: Page, message: string) {
		const toast = page.locator(`.toast-error:has-text("${message}")`)
		await expect(toast).toBeVisible({ timeout: 10000 })
	}
}

export class ConfirmationModal {
	static async confirm(page: Page, modalSelector: string, buttonText: string) {
		const confirmBtn = page.locator(`${modalSelector} button:has-text("${buttonText}")`)
		await confirmBtn.click()
		await page.locator(modalSelector).waitFor({ state: 'hidden' })
	}

	static async expectText(page: Page, modalSelector: string, text: string) {
		const modal = page.locator(modalSelector)
		await expect(modal).toHaveText(text)
	}
}

export const prettify = (s: string) => (s.charAt(0).toUpperCase() + s.slice(1)).replace(/_/g, ' ')
