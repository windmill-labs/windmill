// Assumes the db manager was already opened

import { expect, Locator, Page } from '@playwright/test'
import type { DbFeatures } from '../src/lib/components/apps/components/display/dbtable/dbFeatures'

export class DbManagerPage {
	page: Page
	features: DbFeatures

	constructor(page: Page, features: DbFeatures) {
		this.page = page
		this.features = features
	}

	dbManager = () => this.page.locator('#db-manager-drawer')
	async openCreateTableDrawer(): Promise<TableEditorDrawer> {
		await this.dbManager().locator('button:has-text("New table")').click()
		const tableEditor = new TableEditorDrawer(this.page)
		await expect(tableEditor.tableEditor()).toBeVisible()
		return tableEditor
	}

	async runTest() {
		await expect(this.dbManager()).toBeVisible()
		const page = this.page

		let tableEditor = await this.openCreateTableDrawer()
		await tableEditor.setTableName('friend')
		await tableEditor.addColumn('name', 'TEXT')

		await page.locator('button:has-text("Create table")').click()

		await page.locator('#db-table-editor-confirmation-modal button:has-text("Create")').click()

		const saveSuccessToast = page.locator(`.toast-success:has-text("friend created")`)
		await expect(saveSuccessToast).toBeVisible({ timeout: 10000 })

		const friendTableKey = page.locator('.db-manager-table-key', { hasText: 'friend' })
		await expect(friendTableKey).toBeVisible({ timeout: 10000 })
		await friendTableKey.click()

		// Add a new row
		await this.dbManager().locator('button:has-text("Insert")').click()
		let insertRowDrawer = page.locator('#insert-row-drawer')
		await expect(insertRowDrawer).toBeVisible({ timeout: 10000 })
		await insertRowDrawer.locator('textarea').fill('Alice', { force: true }) // Not sure why force is needed here
		await insertRowDrawer.locator('button:has-text("Insert")').click()

		const rowInsertedToast = page.locator(`.toast-success:has-text("Row inserted")`)
		await expect(rowInsertedToast).toBeVisible({ timeout: 10000 })

		const insertedRow = this.dbManager().locator('.ag-cell-value', { hasText: 'Alice' })
		await expect(insertedRow).toBeVisible({ timeout: 10000 })

		// Edit the row
		await insertedRow.dblclick()
		const cellEditor = this.dbManager().locator('.ag-cell-editor input')
		await cellEditor.fill('Bob')
		await cellEditor.press('Enter')

		let rowUpdatedToast = page.locator(`.toast-success:has-text("Value updated")`)
		await expect(rowUpdatedToast).toBeVisible({ timeout: 10000 })

		const updatedRow = this.dbManager().locator('.ag-cell-value', { hasText: 'Bob' })
		await expect(updatedRow).toBeVisible({ timeout: 10000 })

		const actionsBtn = this.dbManager().locator('#db-manager-table-actions-friend')
		await actionsBtn.click()

		await page.locator('button:has-text("Delete table")').click()

		let deletePermanentlyBtn = page.locator(
			'#db-manager-delete-table-confirmation-modal button:has-text("Delete")'
		)
		await deletePermanentlyBtn.click()

		let tableDeletedToast = page.locator(
			`.toast-success:has-text("Table 'friend' deleted successfully")`
		)
		await expect(tableDeletedToast).toBeVisible({ timeout: 10000 })
	}
}

class TableEditorDrawer {
	page: Page

	constructor(page: Page) {
		this.page = page
	}

	tableEditor = () => this.page.locator('#db-table-editor-drawer')
	columnsTable = () => this.page.locator('#columns-section table')

	async setTableName(name: string) {
		const nameInput = this.tableEditor().locator('label:has-text("Name")').locator('input')
		await nameInput.fill(name)
	}

	async addColumn(columnName: string, columnType: string) {
		const columnsTable = this.columnsTable()
		const addColumnButton = columnsTable.locator('button:has-text("Add")')
		await addColumnButton.click()

		// Set name before creating Column because it's identified by name
		const newColRow = columnsTable.locator('tr').nth(-2)
		const newColNameInput = newColRow.locator('td').nth(0).locator('input')
		await newColNameInput.fill(columnName)

		let column = new Column(this.page, columnsTable, columnName)
		await column.setType(columnType)
		return column
	}
}

class Column {
	columnsTable: Locator
	page: Page
	columnName: string

	constructor(page: Page, columnsTable: Locator, columnName: string) {
		this.page = page
		this.columnsTable = columnsTable
		this.columnName = columnName
	}

	async row(): Promise<Locator> {
		let rows = await this.columnsTable.locator('tr:has(input)').all()
		for (const row of rows) {
			const val = await row.locator('input').first().inputValue()
			if (val === 'name') return row
		}
		throw new Error(`Column with name ${this.columnName} not found`)
	}

	async setName(columnName: string) {
		const newColNameInput = (await this.row()).locator('td').nth(0).locator('input')
		await newColNameInput.fill(columnName)
		this.columnName = columnName
	}

	async setType(columnType: string) {
		const newColTypeSelect = (await this.row()).locator('td').nth(1).locator('input')
		await newColTypeSelect.click()
		await this.page.locator(`li:has(:text-is("${columnType}"))`).click()
	}
}
