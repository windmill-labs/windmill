import { test, Page } from '@playwright/test'
import { runDbManagerAlterTableTest, runDbManagerSimpleCRUDTest } from './DbManagerPage'

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

async function openDataTableDbManager(page: Page) {
	await page.goto('/workspace_settings?tab=windmill_data_tables')
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
