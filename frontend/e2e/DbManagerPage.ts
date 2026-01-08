// Assume the db manager was already opened

import { expect, Locator, Page } from '@playwright/test'
import type { DbFeatures } from '../src/lib/components/apps/components/display/dbtable/dbFeatures'
import { ConfirmationModal, Toast } from './utils'

export async function runDbManagerTests(page: Page, dbFeatures: DbFeatures) {
	let dbManager = new DbManagerPage(page)
	await dbManager.expectToBeVisible()

	// Create table
	const tableEditor = await dbManager.openCreateTableDrawer()
	await tableEditor.setTableName('friend')
	await tableEditor.addColumn('name', 'TEXT')
	await tableEditor.createTable()

	await Toast.expectSuccess(page, 'friend created')

	// Select and work with the table
	await dbManager.selectTable('friend')

	// Insert a row
	const insertDrawer = await dbManager.openInsertDrawer()
	await insertDrawer.fillField('name', 'Alice')
	await insertDrawer.insert()

	await Toast.expectSuccess(page, 'Row inserted')

	// Verify and edit the cell
	const dataGrid = dbManager.dataGrid()
	await dataGrid.expectCellValue('Alice')
	await dataGrid.editCell('Alice', 'Bob')

	await Toast.expectSuccess(page, 'Value updated')
	await dataGrid.expectCellValue('Bob')

	// Delete the table
	const actionsMenu = await dbManager.openActionsMenu('friend')
	await actionsMenu.deleteTable()

	await Toast.expectSuccess(page, "Table 'friend' deleted successfully")
}

export class DbManagerPage {
	page: Page

	constructor(page: Page) {
		this.page = page
	}

	dbManager = () => this.page.locator('#db-manager-drawer')
	expectToBeVisible = () => expect(this.dbManager()).toBeVisible()

	async openCreateTableDrawer(): Promise<TableEditorDrawer> {
		await this.dbManager().locator('button:has-text("New table")').click()
		const tableEditor = new TableEditorDrawer(this.page)
		await expect(tableEditor.tableEditor()).toBeVisible()
		return tableEditor
	}

	async selectTable(tableName: string) {
		const tableKey = this.page.locator('.db-manager-table-key', { hasText: tableName })
		await expect(tableKey).toBeVisible({ timeout: 10000 })
		await tableKey.click()
	}

	async openInsertDrawer(): Promise<InsertRowDrawer> {
		await this.dbManager().locator('button:has-text("Insert")').click()
		return new InsertRowDrawer(this.page)
	}

	dataGrid(): DataGrid {
		return new DataGrid(this.page, this.dbManager())
	}

	async openActionsMenu(tableName: string): Promise<TableActionsMenu> {
		const actionsBtn = this.dbManager().locator(`#db-manager-table-actions-${tableName}`)
		await actionsBtn.click()
		return new TableActionsMenu(this.page)
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

	async createTable() {
		await this.page.locator('button:has-text("Create table")').click()
		await ConfirmationModal.confirm(this.page, '#db-table-editor-confirmation-modal', 'Create')
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

class InsertRowDrawer {
	page: Page

	constructor(page: Page) {
		this.page = page
	}

	drawer = () => this.page.locator('#insert-row-drawer')

	async fillField(fieldName: string, value: string) {
		await expect(this.drawer()).toBeVisible({ timeout: 10000 })
		// For now, assumes single field - could be enhanced to handle multiple fields
		await this.drawer().locator('textarea').fill(value, { force: true })
	}

	async insert() {
		await this.drawer().locator('button:has-text("Insert")').click()
	}
}

class DataGrid {
	page: Page
	dbManager: Locator

	constructor(page: Page, dbManager: Locator) {
		this.page = page
		this.dbManager = dbManager
	}

	async expectCellValue(value: string) {
		const cell = this.dbManager.locator('.ag-cell-value', { hasText: value })
		await expect(cell).toBeVisible({ timeout: 10000 })
	}

	async editCell(oldValue: string, newValue: string) {
		const cell = this.dbManager.locator('.ag-cell-value', { hasText: oldValue })
		await cell.dblclick()

		const cellEditor = this.dbManager.locator('.ag-cell-editor input')
		await cellEditor.fill(newValue)
		await cellEditor.press('Enter')
	}
}

class TableActionsMenu {
	page: Page

	constructor(page: Page) {
		this.page = page
	}

	async deleteTable() {
		await this.page.locator('button:has-text("Delete table")').click()
		await ConfirmationModal.confirm(
			this.page,
			'#db-manager-delete-table-confirmation-modal',
			'Delete'
		)
	}
}
