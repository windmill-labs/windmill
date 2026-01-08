import { test, expect, Page } from '@playwright/test'
import { runDbManagerSimpleCRUDTest } from './DbManagerPage'
import { getDbFeatures } from '../src/lib/components/apps/components/display/dbtable/dbFeatures'
import { Toast } from './utils'

test('setup a datatable and ensure db manager works', async ({ page }) => {
	await setupNewDataTableAndOpenDbManager(page)
	await runDbManagerSimpleCRUDTest(page, dbFeatures)
})

async function setupNewDataTable(page: Page): Promise<{ datatableId: string }> {
	// Generate unique ID with timestamp
	const timestamp = Date.now()
	const datatableId = `datatable_${timestamp}`

	// Navigate to workspace settings data tables tab
	await page.goto('/workspace_settings?tab=windmill_data_tables')

	// Click on 'New Data Table' button
	const newDataTableButton = page.locator('button:has-text("New Data Table")')
	await newDataTableButton.waitFor({ state: 'visible' })
	await newDataTableButton.click()

	// Find the second-to-last row in the table (last row contains the button)
	const table = page.locator('table')
	await table.waitFor({ state: 'visible' })

	const lastRow = table.locator('tr').nth(-2)
	await lastRow.waitFor({ state: 'visible' })

	// Fill the name input with generated ID
	const nameInput = lastRow.locator('input[id="name"]')
	await nameInput.waitFor({ state: 'visible' })
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
	Toast.expectSuccess(page, 'Setup successful')

	const closeModalBtn = page.locator('button[id="modal-close-button"]')
	await closeModalBtn.click()

	const saveBtn = page.locator('button:has-text("Save")')
	await saveBtn.click()

	// Verify success toast appears
	Toast.expectSuccess(page, 'saved successfully')

	return { datatableId }
}

async function setupNewDataTableAndOpenDbManager(page: Page) {
	await setupNewDataTable(page)
	const table = page.locator('table')
	const lastRow = table.locator('tr').nth(-2)
	lastRow.locator('button:has-text("Manage")').click()
}

const dbFeatures = getDbFeatures({
	type: 'database',
	resourceType: 'postgresql',
	resourcePath: ''
})
