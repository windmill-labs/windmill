// Assumes the db manager was already opened

import { expect, Page } from '@playwright/test'
import type { DbFeatures } from '../src/lib/components/apps/components/display/dbtable/dbFeatures'

export class DbManagerPage {
	page: Page
	features: DbFeatures
	constructor(page: Page, features: DbFeatures) {
		this.page = page
		this.features = features
	}

	async runTest() {
		const page = this.page
		// Wait for DB Manager drawer to appear
		const dbManager = page.locator('#db-manager-drawer')
		await expect(dbManager).toBeVisible()
		await dbManager.locator('button:has-text("New table")').click()
		const tableEditor = page.locator('#db-table-editor-drawer')
		await expect(tableEditor).toBeVisible()

		const nameInput = page.locator('label:has-text("Name")').locator('input')
		await nameInput.fill('friend')

		const columnsTable = page.locator('#columns-section table')
		const addColumnButton = columnsTable.locator('button:has-text("Add")')
		await addColumnButton.click()

		const newColRow = columnsTable.locator('tr').nth(-2)
		const newColNameInput = newColRow.locator('td').nth(0).locator('input')
		await newColNameInput.fill('name')

		const newColTypeSelect = newColRow.locator('td').nth(1).locator('input')
		await newColTypeSelect.click()

		await page.locator('li:has-text("TEXT")').first().click()

		await page.locator('button:has-text("Create table")').click()

		await page.locator('#db-table-editor-confirmation-modal button:has-text("Create")').click()

		// Verify success toast appears
		const saveSuccessToast = page.locator('text=friend created!')
		await expect(saveSuccessToast).toBeVisible({ timeout: 10000 })

		const friendTableKey = page.locator('.db-manager-table-key', { hasText: 'friend' })
		await expect(friendTableKey).toBeVisible({ timeout: 10000 })
		await friendTableKey.click()

		// Add a new row
		await dbManager.locator('button:has-text("Insert")').click()
		let insertRowDrawer = page.locator('#insert-row-drawer')
		await expect(insertRowDrawer).toBeVisible({ timeout: 10000 })
		await insertRowDrawer.locator('textarea').fill('Alice', { force: true }) // Not sure why force is needed here
		await insertRowDrawer.locator('button:has-text("Insert")').click()

		const rowInsertedToast = page.locator('text=Row inserted')
		await expect(rowInsertedToast).toBeVisible({ timeout: 10000 })

		const insertedRow = dbManager.locator('.ag-cell-value', { hasText: 'Alice' })
		await expect(insertedRow).toBeVisible({ timeout: 10000 })

		// Edit the row
		await insertedRow.dblclick()
		const cellEditor = dbManager.locator('.ag-cell-editor input')
		await cellEditor.fill('Bob')
		await cellEditor.press('Enter')

		const rowUpdatedToast = page.locator('text=Value updated')
		await expect(rowUpdatedToast).toBeVisible({ timeout: 10000 })
		const updatedRow = dbManager.locator('.ag-cell-value', { hasText: 'Bob' })
		await expect(updatedRow).toBeVisible({ timeout: 10000 })

		const actionsBtn = dbManager.locator('#db-manager-table-actions-friend')
		await actionsBtn.click()

		await page.locator('button:has-text("Delete table")').click()

		let deletePermanentlyBtn = page.locator(
			'#db-manager-delete-table-confirmation-modal button:has-text("Delete")'
		)
		await deletePermanentlyBtn.click()

		await expect(page.locator("text=Table 'friend' deleted successfully")).toBeVisible({
			timeout: 10000
		})
	}
}
