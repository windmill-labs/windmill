import { test, expect, Page } from '@playwright/test'
import { DbManagerPage } from './DbManagerPage'
import { getDbFeatures } from '../src/lib/components/apps/components/display/dbtable/dbFeatures'
import { DbType } from '../src/lib/components/dbTypes'
import os from 'os'

const resourceByDbType: Record<DbType, object> = {
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
	duckdb: {}, // TODO
	bigquery: {}, // TODO
	ms_sql_server: {}, // TODO
	snowflake: {} // TODO
}

async function setupNewResource(
	page: Page,
	resourceType: DbType
): Promise<{ resourceName: string }> {
	// Generate unique ID with timestamp
	const timestamp = Date.now()
	const resourceName = `${resourceType}_${timestamp}`

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

	// Verify success toast appears
	const successToast = page.locator(`.toast-success:has-text("Saved resource")`)
	await expect(successToast).toBeVisible({ timeout: 10000 })

	return { resourceName }
}

test('setup a postgresql resource and ensure db manager works', async ({ page }) => {
	let { resourceName } = await setupNewResource(page, 'postgresql')

	const resourceRow = page.locator(`table tr:has-text("${resourceName}")`)
	await expect(resourceRow).toBeVisible()

	const manageButton = resourceRow.locator('button:has-text("Manage")')
	await manageButton.click()

	let dbManagerPage = new DbManagerPage(
		page,
		getDbFeatures({ type: 'database', resourceType: 'postgresql', resourcePath: '' })
	)
	await dbManagerPage.runTest()
})
