import { test, expect, Page } from '@playwright/test'

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
	const successToast = page.locator('text=Setup successful')
	await expect(successToast).toBeVisible({ timeout: 10000 })

	const closeModalBtn = page.locator('button[id="modal-close-button"]')
	await closeModalBtn.click()

	const saveBtn = page.locator('button:has-text("Save")')
	await saveBtn.click()

	// Verify success toast appears
	const saveSuccessToast = page.locator('text=saved successfully')
	await expect(saveSuccessToast).toBeVisible({ timeout: 10000 })

	return { datatableId }
}

test('create new data table, add a new table, CRUD rows and delete the table', async ({ page }) => {
	let { datatableId } = await setupNewDataTable(page)
	const table = page.locator('table')
	const lastRow = table.locator('tr').nth(-2)
	lastRow.locator('button:has-text("Manage")').click()

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
})
