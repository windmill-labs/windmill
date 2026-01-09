import { test, expect, Page } from '@playwright/test'
import { runDbManagerAlterTableTest, runDbManagerSimpleCRUDTest } from './DbManagerPage'
import { DbType } from '../src/lib/components/dbTypes'
import { Toast, prettify } from './utils'

test.describe('Database resources', () => {
	for (const dbType of ['postgresql', 'mysql', 'bigquery', 'ms_sql_server', 'snowflake'] as const) {
		test.describe(prettify(dbType), () => {
			test(`simple CRUD with DB Manager`, async ({ page }) => {
				await setupNewResourceAndOpenDbManager(page, dbType)
				await runDbManagerSimpleCRUDTest(page, dbType)
			})
			test(`Alter table with DB Manager`, async ({ page }) => {
				await setupNewResourceAndOpenDbManager(page, dbType)
				await runDbManagerAlterTableTest(page, dbType)
			})
		})
	}
})

test.describe('Data tables', () => {
	test('simple CRUD with DB Manager', async ({ page }) => {
		await openDataTableDbManager(page)
		await runDbManagerSimpleCRUDTest(page, 'postgresql')
	})
	test('Alter table with DB Manager', async ({ page }) => {
		await openDataTableDbManager(page)
		await runDbManagerAlterTableTest(page, 'postgresql')
	})
})

test.describe('Ducklake', () => {
	test('simple CRUD with DB Manager', async ({ page }) => {
		await openDucklakeDbManager(page)
		await runDbManagerSimpleCRUDTest(page, 'ducklake')
	})
	test('Alter table with DB Manager', async ({ page }) => {
		await openDucklakeDbManager(page)
		await runDbManagerAlterTableTest(page, 'ducklake')
	})
})

async function openDucklakeDbManager(page: Page) {
	// await page.goto('/workspace_settings?tab=windmill_lfs')]
	// // Check if ducklake already exists
	// let table = page.locator('table')
	// await table.waitFor({ state: 'visible' })
	// let rows = await table.locator('tr:has(input[id="name"])').all()
	// for (const row of rows) {
	// 	const val = await row.locator('input[id="name"]').inputValue()
	// 	// Don't setup again if it already exists (saving the settings creates race condition)
	// 	if (val === datatableId) return
	// }
}

async function openDataTableDbManager(page: Page) {
	await page.goto('/workspace_settings?tab=windmill_data_tables')
	await setupNewDataTable(page, `datatable_${process.env.TEST_UNIQUE_ID}`)
	const datatableId = `datatable_${process.env.TEST_UNIQUE_ID}`
	let table = page.locator('table')
	await table.waitFor({ state: 'visible' })
	let rows = await table.locator('tr:has(input[id="name"])').all()
	for (const row of rows) {
		const val = await row.locator('input[id="name"]').inputValue()
		if (val === datatableId) {
			await row.locator('button:has-text("Manage")').click()
			return
		}
	}
	throw new Error(`Could not find datatable row ${datatableId}`)
}

declare const process: any // ignore TS errors

async function setupNewDataTable(page: Page, datatableId: string) {
	// Check if datatable already exists
	let table = page.locator('table')
	await table.waitFor({ state: 'visible' })
	let rows = await table.locator('tr:has(input[id="name"])').all()
	for (const row of rows) {
		const val = await row.locator('input[id="name"]').inputValue()
		// Don't setup again if it already exists (saving the settings creates race condition)
		if (val === datatableId) return
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

const resourceByDbType = {
	postgresql: {
		host: 'postgres_e2e',
		port: 5432,
		dbname: 'test_db',
		user: 'test_user',
		password: 'postgres_password',
		sslmode: 'disable'
	},
	mysql: {
		host: 'mysql_e2e',
		port: 3306,
		user: 'test_user',
		database: 'test_db',
		password: 'test_password',
		ssl: false
	},
	oracle: {
		user: 'test_user',
		password: 'test_password',
		database: 'oracle_e2e:1521/test_db'
	},
	ms_sql_server: {
		host: 'mssql_e2e',
		user: 'sa',
		password: 'MsSql_Pass123!',
		port: 1433,
		dbname: 'master',
		instance_name: '',
		trust_cert: true,
		ca_cert: '',
		encrypt: true
	},
	bigquery: {}, // TODO
	snowflake: {} // TODO
} as const

async function setupNewResource(
	page: Page,
	resourceType: DbType
): Promise<{ resourceName: string }> {
	const resourceName = `${resourceType}_e2e}`

	await page.goto('/resources')

	await page.locator('table').waitFor({ state: 'visible' })
	if (await page.locator(`table tr:has-text("${resourceName}")`).isVisible())
		return { resourceName }

	const newDataTableButton = page.locator('button:text-is("Add resource")')
	await newDataTableButton.click()

	const addResourceDrawer = page.locator('#add-resource-drawer')
	await expect(addResourceDrawer).toBeVisible()

	const resourceTypeBtn = addResourceDrawer.locator(`button:has-text("${resourceType}")`)
	await resourceTypeBtn.waitFor({ state: 'visible' })
	await resourceTypeBtn.click()

	const asJsonToggle = addResourceDrawer.locator('label.as-json-toggle')
	await expect(asJsonToggle).toBeVisible()
	await asJsonToggle.check()

	await addResourceDrawer.locator('input#path').fill(resourceName)

	const resourceObj = resourceByDbType[resourceType]
	const jsonEditor = page.locator('.simple-editor .view-lines')
	await expect(jsonEditor).toBeVisible()
	await jsonEditor.click({ clickCount: 4 }) // Select all existing text
	await page.evaluate((c) => navigator.clipboard.writeText(c), JSON.stringify(resourceObj))
	await page.keyboard.press('ControlOrMeta+V')

	const saveButton = addResourceDrawer.locator('button:has-text("Save")')
	await saveButton.click()

	await Toast.expectSuccess(page, 'Saved resource')

	return { resourceName }
}

async function setupNewResourceAndOpenDbManager(page: Page, dbType: DbType) {
	let { resourceName } = await setupNewResource(page, dbType)

	const resourceRow = page.locator(`table tr:has-text("${resourceName}")`)
	await expect(resourceRow).toBeVisible()

	const manageButton = resourceRow.locator('button:has-text("Manage")')
	await manageButton.click()
}
