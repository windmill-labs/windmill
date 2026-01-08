import test, { expect, Page } from '@playwright/test'
import { Toast } from './utils'

test('setup a datatable', async ({ page }) => {
	if (!process.env.TEST_UNIQUE_ID) throw new Error('TEST_UNIQUE_ID was not generated')
	await setupNewDataTable(page, `datatable_${process.env.TEST_UNIQUE_ID}`)
})

// This saves the datatable settings which will create race conditions with other tests
async function setupNewDataTable(page: Page, datatableId: string) {
	// Navigate to workspace settings data tables tab
	await page.goto('/workspace_settings?tab=windmill_data_tables')

	// Check if datatable already exists
	let table = page.locator('table')
	await table.waitFor({ state: 'visible' })
	let rows = await table.locator('tr:has(input[id="name"])').all()
	for (const row of rows) {
		const val = await row.locator('input[id="name"]').inputValue()
		if (val === datatableId) return // Don't setup again if it already exists
	}

	// Click on 'New Data Table' button
	const newDataTableButton = page.locator('button:has-text("New Data Table")')
	await newDataTableButton.click()

	// Find the second-to-last row in the table (last row contains the button)
	const lastRow = table.locator('tr').nth(-2)

	// Fill the name input with generated ID
	const nameInput = lastRow.locator('input[id="name"]')
	await nameInput.fill(datatableId)

	// Verify database type is 'Instance'
	const databaseTypeSelect = lastRow.locator('input[id="database-type-select"]')
	await expect(databaseTypeSelect).toHaveValue('Instance')

	// Click on custom instance DB select and add new database
	const customInstanceDbSelect = lastRow.locator('input[id="custom-instance-db-select"]')
	await customInstanceDbSelect.waitFor({ state: 'visible' })
	await customInstanceDbSelect.click()
	await customInstanceDbSelect.fill(datatableId)

	// Click on the "Add new:" button
	const addNewButton = page.locator(`button:has-text("Add new: ")`)
	await addNewButton.waitFor({ state: 'visible' })
	await addNewButton.click()

	// Click on Setup button
	const setupButton = lastRow.locator('button:has-text("Setup")')
	await setupButton.waitFor({ state: 'visible' })
	await setupButton.click()

	// Wait for popover to appear and click run setup button
	const runSetupButton = page.locator('button[id="run-custom-instance-db-setup-button"]')
	await runSetupButton.waitFor({ state: 'visible' })
	await runSetupButton.click()

	const confirmBtn = page.locator('button:has-text("Setup database")')
	await confirmBtn.waitFor({ state: 'visible' })
	await confirmBtn.click()

	// Verify success toast appears
	await Toast.expectSuccess(page, 'Setup successful')

	const closeModalBtn = page.locator('button[id="modal-close-button"]')
	await closeModalBtn.click()

	const saveBtn = page.locator('button:has-text("Save")')
	await saveBtn.click()

	if (await page.locator('text=Some databases are not setup').isVisible()) {
		await page.locator('button:has-text("Save anyway")').click()
	}

	// Verify success toast appears
	await Toast.expectSuccess(page, 'saved successfully')

	return { datatableId }
}

declare const process: any // ignore TS errors
