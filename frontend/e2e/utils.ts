import { expect, Locator, Page } from '@playwright/test'

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

export class Dropdown {
	static getOption(page: Page, optionText: string, { exact = true } = {}) {
		if (exact) return page.locator(`.select-dropdown-open li:has(:text-is("${optionText}"))`)
		else return page.locator(`.select-dropdown-open li:has-text("${optionText}")`)
	}
	static async selectOption(
		page: Page,
		selectInput: Locator,
		optionText: string,
		{ exact = true } = {}
	) {
		await selectInput.click()
		await selectInput.fill(optionText)
		const option = this.getOption(page, optionText, { exact })
		await option.click()
	}
}
