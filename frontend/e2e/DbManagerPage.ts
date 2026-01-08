// Assume the db manager was already opened

import { expect, Locator, Page } from '@playwright/test'
import {
	getDbFeatures,
	type DbFeatures
} from '../src/lib/components/apps/components/display/dbtable/dbFeatures'
import { ConfirmationModal, Toast } from './utils'
import { DbInput, DbType } from '../src/lib/components/dbTypes'

export async function runDbManagerSimpleCRUDTest(page: Page, db: _DbType) {
	let dbManager = new DbManagerPage(page)
	await dbManager.expectToBeVisible()

	let friendTableName = `friend_${Date.now()}`

	// Create table
	const tableEditor = await dbManager.openCreateTableDrawer()
	await tableEditor.setTableName(friendTableName)
	await tableEditor.addColumn('name', 'TEXT')
	await tableEditor.createTable()

	await Toast.expectSuccess(page, `${friendTableName} created`)

	// Select and work with the table
	await dbManager.selectTable(friendTableName)

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
	const actionsMenu = await dbManager.openActionsMenu(friendTableName)
	await actionsMenu.deleteTable()

	await Toast.expectSuccess(page, `Table '${friendTableName}' deleted successfully`)
}

export async function runDbManagerAlterTableTest(page: Page, dbType: _DbType) {
	let dbFeatures = getDbFeatures(getDbInput(dbType))
	let dbManager = new DbManagerPage(page)
	await dbManager.expectToBeVisible()

	let timestamp = Date.now()

	// Create friend table
	let friendTableName = `friend_${timestamp}`
	let tableEditor = await dbManager.openCreateTableDrawer()
	await tableEditor.setTableName(friendTableName)
	await new Column(page, tableEditor.columnsSection(), 'id').delete() // default id column
	let friendIdCol = await tableEditor.addColumn('id', 'INT')
	if (dbFeatures.primaryKeys) {
		friendIdCol.setPrimaryKey(true)
	} else {
		await expect(await friendIdCol.primaryKeyCheckbox()).toBeHidden()
	}
	await tableEditor.addColumn('name', 'TEXT')
	await tableEditor.addColumn('created_at', dbType === 'ms_sql_server' ? 'DATETIME2' : 'TIMESTAMP')
	await tableEditor.createTable()
	await Toast.expectSuccess(page, `${friendTableName} created`)

	// Create message table
	let messageTableName = `message_${timestamp}`
	tableEditor = await dbManager.openCreateTableDrawer()
	await tableEditor.setTableName(messageTableName)
	await new Column(page, tableEditor.columnsSection(), 'id').delete() // default id column
	let messageIdCol = await tableEditor.addColumn('id', 'INT')
	if (dbFeatures.primaryKeys) messageIdCol.setPrimaryKey(true)
	await tableEditor.addColumn('friend_id', 'INT')
	await tableEditor.addColumn('content', 'TEXT')
	await tableEditor.addColumn('created_at', dbType === 'ms_sql_server' ? 'DATETIME2' : 'TIMESTAMP')
	if (dbFeatures.foreignKeys) {
		await tableEditor.addForeignKey(friendTableName, 'friend_id', 'id', {
			onDelete: 'CASCADE',
			onUpdate: 'CASCADE'
		})
	} else {
		expect(tableEditor.foreignKeySection()).toBeHidden()
	}
	await tableEditor.createTable()
	await Toast.expectSuccess(page, `${messageTableName} created`)

	// Alter message table
	let actionsMenu = await dbManager.openActionsMenu(messageTableName)
	await actionsMenu.alterTable()
	tableEditor = new TableEditorDrawer(page)
	await tableEditor.setTableName(`posts_${timestamp}`)
	await new Column(page, tableEditor.columnsSection(), 'id').delete()
	await new Column(page, tableEditor.columnsSection(), 'friend_id').setName('person_id')
	await new Column(page, tableEditor.columnsSection(), 'created_at').setType('INT')
	await new Column(page, tableEditor.columnsSection(), 'created_at').setName('created_timestamp')
	await tableEditor.alterTable()
	await Toast.expectSuccess(page, `posts updated successfully`)
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
		await expect(actionsBtn).toBeVisible({ timeout: 10000 })
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
	columnsSection = () => this.tableEditor().locator('#columns-section')
	foreignKeySection = () => this.page.locator('#foreign-keys-section')

	async setTableName(name: string) {
		const nameInput = this.tableEditor().locator('label:has-text("Name")').locator('input')
		await nameInput.fill(name)
	}

	async addColumn(columnName: string, columnType: string) {
		const columnsSection = this.columnsSection()
		const addColumnButton = columnsSection.locator('button:has-text("Add")')
		await addColumnButton.click()

		// Set name before creating Column because it's identified by name
		const newColRow = columnsSection.locator('tr').nth(-2)
		const newColNameInput = newColRow.locator('td').nth(0).locator('input')
		await newColNameInput.fill(columnName)

		let column = new Column(this.page, columnsSection, columnName)
		await column.setType(columnType)
		return column
	}

	async createTable() {
		await this.tableEditor().locator('button:has-text("Create table")').click()
		await ConfirmationModal.confirm(this.page, '#db-table-editor-confirmation-modal', 'Create')
	}

	async alterTable() {
		await this.tableEditor().locator('button:has-text("Alter table")').click()
		await ConfirmationModal.confirm(this.page, '#db-table-editor-confirmation-modal', 'Alter')
	}

	async addForeignKey(
		referencedTable: string,
		fromCol: string,
		toCol: string,
		options?: { onDelete?: string; onUpdate?: string }
	) {
		const fkSection = this.foreignKeySection()
		const addFkButton = fkSection.locator('button:has-text("Add")')
		await addFkButton.click()
		const lastFkRow = fkSection.locator('tr').nth(-2)

		const tableSelect = lastFkRow.locator('input.fk-table-select')
		await tableSelect.click()
		await this.page.locator(`.select-dropdown-open li:has(:text-is("${referencedTable}"))`).click()

		const fromColSelect = lastFkRow.locator('.fk-source-col-select')
		await fromColSelect.click()
		await this.page.locator(`.select-dropdown-open li:has(:text-is("${fromCol}"))`).click()

		const toColSelect = lastFkRow.locator('.fk-target-col-select')
		await toColSelect.click()
		await this.page.locator(`.select-dropdown-open li:has(:text-is("${toCol}"))`).click()

		const fkSettings = lastFkRow.locator('.fk-settings-btn')
		if (options?.onDelete || options?.onUpdate) {
			await fkSettings.click()
			if (options?.onDelete) {
				const onDeleteSelect = this.page.locator('select.fk-on-delete-select')
				await onDeleteSelect.selectOption({ label: options.onDelete })
			}
			if (options?.onUpdate) {
				const onUpdateSelect = this.page.locator('select.fk-on-update-select')
				await onUpdateSelect.selectOption({ label: options.onUpdate })
			}
			// Close the popover
			await fkSettings.click()
		}
	}
}

class Column {
	columnsSection: Locator
	page: Page
	columnName: string

	constructor(page: Page, columnsSection: Locator, columnName: string) {
		this.page = page
		this.columnsSection = columnsSection
		this.columnName = columnName
	}

	async row(): Promise<Locator> {
		let rows = await this.columnsSection.locator('tr:has(input)').all()
		for (const row of rows) {
			const val = await row.locator('input').first().inputValue()
			if (val === this.columnName) return row
		}
		throw new Error(`Column with name ${this.columnName} not found`)
	}

	primaryKeyCheckbox = async () => (await this.row()).locator('input.primary-key-checkbox')

	async setName(columnName: string) {
		const newColNameInput = (await this.row()).locator('td').nth(0).locator('input')
		await newColNameInput.fill(columnName)
		this.columnName = columnName
	}

	async setType(columnType: string) {
		const newColTypeSelect = (await this.row()).locator('td').nth(1).locator('input')
		await newColTypeSelect.click()
		await this.page.locator(`.select-dropdown-open li:has(:text-is("${columnType}"))`).click()
	}

	async delete() {
		const deleteBtn = (await this.row()).locator('button.delete-column-btn')
		await deleteBtn.click()
	}

	async setPrimaryKey(isPrimaryKey: boolean) {
		let primaryKeyCheckbox = await this.primaryKeyCheckbox()
		const isChecked = await primaryKeyCheckbox.isChecked()
		if (isChecked !== isPrimaryKey) {
			await primaryKeyCheckbox.click()
		}
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

	async alterTable() {
		await this.page.locator('button:has-text("Alter table")').click()
	}
}

type _DbType = Exclude<DbType, 'duckdb'> | 'ducklake'
function getDbInput(dbType: _DbType): DbInput {
	if (dbType === 'ducklake') {
		return { type: 'ducklake', ducklake: '' }
	} else {
		return { type: 'database', resourceType: dbType, resourcePath: '' }
	}
}
