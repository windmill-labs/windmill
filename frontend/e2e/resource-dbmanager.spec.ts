import { test, expect, Page } from '@playwright/test'
import { runDbManagerSimpleCRUDTest } from './DbManagerPage'
import { getDbFeatures } from '../src/lib/components/apps/components/display/dbtable/dbFeatures'
import { DbType } from '../src/lib/components/dbTypes'
import { Toast } from './utils'

test.describe('DB Manager', () => {
	for (const dbType of ['postgresql', 'mysql', 'bigquery', 'ms_sql_server', 'snowflake'] as const) {
		const dbFeatures = getDbFeatures({ type: 'database', resourceType: dbType, resourcePath: '' })
		test(`setup a ${dbType} resource and ensure db manager works`, async ({ page }) => {
			await setupNewResourceAndOpenDbManager(page, dbType)
			await runDbManagerSimpleCRUDTest(page, dbFeatures)
		})
	}
})

const resourceByDbType = {
	postgresql: {
		host: 'localhost',
		port: 5432,
		dbname: 'testing',
		user: 'postgres',
		password: 'changeme',
		sslmode: 'disable',
		root_certificate_pem: ''
	},
	mysql: {}, // TODO
	bigquery: {}, // TODO
	ms_sql_server: {}, // TODO
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
	// We do copy paste to avoid issues with monaco helping with autocomplete and messing up the test
	await page.context().grantPermissions(['clipboard-read', 'clipboard-write'])
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
