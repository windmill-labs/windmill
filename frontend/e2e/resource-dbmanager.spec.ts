import { test, expect, Page } from '@playwright/test'
import { runDbManagerSimpleCRUDTest } from './DbManagerPage'
import { DbType } from '../src/lib/components/dbTypes'
import { Toast } from './utils'

test.describe('DB Manager', () => {
	for (const dbType of ['postgresql', 'mysql', 'bigquery', 'ms_sql_server', 'snowflake'] as const) {
		test(`setup a ${dbType} resource and test simple CRUD with DB Manager`, async ({ page }) => {
			await setupNewResourceAndOpenDbManager(page, dbType)
			await runDbManagerSimpleCRUDTest(page, dbType)
		})
	}
})

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
	// Generate unique ID with timestamp
	const resourceName = `${resourceType}_${Date.now()}`

	await page.goto('/resources')

	const newDataTableButton = page.locator('button:text-is("Add resource")')
	await newDataTableButton.waitFor({ state: 'visible' })
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
