// Assume the db manager was already opened

import { expect, Locator, Page } from '@playwright/test'
import { getDbFeatures } from '../src/lib/components/apps/components/display/dbtable/dbFeatures'
import { ConfirmationModal, Dropdown, Toast } from './utils'
import { DbInput, DbType } from '../src/lib/components/dbTypes'
import { DB_TYPES } from '../src/lib/consts'

export async function runDbManagerSimpleCRUDTest(page: Page, dbType: _DbType) {
	let dbManager = new DbManagerPage(page)
	await dbManager.expectToBeVisible()

	let friendTableName = identifier(dbType, `friend_${Date.now()}`)

	// Create table
	const tableEditor = await dbManager.openCreateTableDrawer()
	await tableEditor.setTableName(friendTableName)
	await tableEditor.addColumn(identifier(dbType, 'name'), getDbDatatype(dbType, 'TEXT'))
	await tableEditor.getColumn('id').delete() // remove default id column
	await tableEditor.createTable()

	await Toast.expectSuccess(page, `${friendTableName} created`)

	// Select and work with the table
	await dbManager.selectTable(friendTableName)

	// Insert a row
	const insertDrawer = await dbManager.openInsertDrawer()
	await insertDrawer.fillField(identifier(dbType, 'name'), 'Alice')
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
	let timestamp = Date.now()
	await dbManager.expectToBeVisible()

	if (dbFeatures.schemas) {
		const schemaName = identifier(dbType, `schema_${timestamp}`)
		await dbManager.setCurrentSchema(schemaName, { create: true })
	}

	// Create friend table
	let friendTableName = identifier(dbType, `friend_${timestamp}`)
	let tableEditor = await dbManager.openCreateTableDrawer()
	await tableEditor.setTableName(friendTableName)
	let friendIdCol = await tableEditor.getColumn('id') // deafult id column
	await friendIdCol.setType(getDbDatatype(dbType, 'INT'))
	if (dbFeatures.primaryKeys) {
		friendIdCol.setPrimaryKey(true)
	} else {
		await expect(await friendIdCol.primaryKeyCheckbox()).toBeHidden()
	}
	await tableEditor.addColumn(identifier(dbType, 'name'), getDbDatatype(dbType, 'TEXT'))
	await tableEditor.addColumn(identifier(dbType, 'created_at'), getDbDatatype(dbType, 'TIMESTAMP'))
	await tableEditor.createTable()
	await Toast.expectSuccess(page, `${friendTableName} created`)
	await dbManager.selectTable(friendTableName) // Ensure the view refreshed

	// Create message table
	let messageTableName = identifier(dbType, `message_${timestamp}`)
	tableEditor = await dbManager.openCreateTableDrawer()
	await tableEditor.setTableName(messageTableName)
	let messageIdCol = await tableEditor.getColumn('id') // deafult id column
	await messageIdCol.setType(getDbDatatype(dbType, 'INT'))
	if (dbFeatures.primaryKeys) messageIdCol.setPrimaryKey(true)
	await tableEditor.addColumn(identifier(dbType, 'friend_id'), getDbDatatype(dbType, 'INT'))
	let contentColumn = await tableEditor.addColumn(
		identifier(dbType, 'content'),
		getDbDatatype(dbType, 'TEXT')
	)
	await contentColumn.setSettings({ nullable: true })
	await tableEditor.addColumn(identifier(dbType, 'created_at'), getDbDatatype(dbType, 'TIMESTAMP'))
	if (dbFeatures.foreignKeys) {
		await tableEditor.addForeignKey(
			friendTableName,
			identifier(dbType, 'friend_id'),
			identifier(dbType, 'id'),
			dbFeatures.enforcedForeignKeys ? { onDelete: 'Cascade', onUpdate: 'Cascade' } : undefined
		)
	} else {
		await expect(tableEditor.foreignKeySection()).toBeHidden()
	}
	await tableEditor.createTable()
	await Toast.expectSuccess(page, `${messageTableName} created`)
	await page.waitForTimeout(100)

	// Alter message table
	await (await dbManager.openActionsMenu(messageTableName)).alterTable()
	await tableEditor.expectNoChangesDetected()
	let postsTableName = identifier(dbType, `posts_${timestamp}`)
	let friendCol = tableEditor.getColumn(identifier(dbType, 'friend_id'))
	let createdAtCol = tableEditor.getColumn(identifier(dbType, 'created_at'))
	let idCol = tableEditor.getColumn(identifier(dbType, 'id'))

	// Predicate checks
	if (dbFeatures.primaryKeys) {
		await idCol.checkPrimaryKeyIs(true)
		await createdAtCol.checkPrimaryKeyIs(false)
		await contentColumn.checkPrimaryKeyIs(false)
		await friendCol.checkPrimaryKeyIs(false)
		await friendCol.checkSettingsIs({
			nullable: false,
			defaultValue: dbFeatures.defaultValues ? '' : undefined
		})
	}

	// Apply alterations
	await tableEditor.setTableName(postsTableName)
	await idCol.delete()
	await friendCol.setName(identifier(dbType, 'person_id'))
	if (dbType !== 'bigquery' && dbType !== 'snowflake') {
		await friendCol.setType(getDbDatatype(dbType, 'BIGINT'))
	}
	if (dbType !== 'snowflake') {
		// Snowflake does not support altering default values
		await friendCol.setSettings({
			defaultValue: dbFeatures.defaultValues ? '123' : undefined,
			nullable: false
		})
	}
	if (dbFeatures.primaryKeys && dbType !== 'bigquery') {
		// Bigquery cannot rename a table with primary keys
		await friendCol.setPrimaryKey(true)
		await createdAtCol.setPrimaryKey(true)
	}
	if (dbFeatures.foreignKeys) await tableEditor.deleteForeignKey()
	await tableEditor.alterTable()
	await Toast.expectSuccess(page, `${messageTableName} updated`) // uses old table name
	await page.waitForTimeout(100)

	// Verify alterations
	await dbManager.selectTable(postsTableName) // Ensure the view refreshed
	await (await dbManager.openActionsMenu(postsTableName)).alterTable()
	await tableEditor.expectNoChangesDetected()
	tableEditor = new TableEditorDrawer(page)
	await idCol.checkNotExists()
	await friendCol.checkNameIs(identifier(dbType, 'person_id'))
	if (dbType !== 'bigquery' && dbType !== 'snowflake') {
		await friendCol.checkTypeIs(getDbDatatype(dbType, 'BIGINT'))
	}
	if (dbFeatures.defaultValues && dbType !== 'snowflake') {
		await friendCol.checkSettingsIs({ defaultValue: /123/ })
	}
	await createdAtCol.checkTypeIs(
		dbType === 'snowflake' ? 'TIMESTAMP_NTZ' : getDbDatatype(dbType, 'TIMESTAMP')
	)
	await createdAtCol.checkNameIs(identifier(dbType, 'created_at'))
	if (dbFeatures.primaryKeys && dbType !== 'bigquery') {
		await friendCol.checkPrimaryKeyIs(true)
		await createdAtCol.checkPrimaryKeyIs(true)
		await contentColumn.checkPrimaryKeyIs(false)
	}
}

export class DbManagerPage {
	page: Page

	constructor(page: Page) {
		this.page = page
	}

	async setCurrentSchema(schemaName: string, options?: { create?: boolean }) {
		const schemaSelect = this.dbManager().locator('input[id="db-schema-select"]')
		await schemaSelect.click()
		await schemaSelect.fill(schemaName)
		const option = Dropdown.getOption(this.page, schemaName).or(
			Dropdown.getOption(this.page, 'Add new', { exact: false })
		)
		await option.click()
		if (options?.create) {
			await ConfirmationModal.confirm(this.page, '#db-create-schema-confirmation-modal', 'Create')
		}
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
		await expect(tableKey).toBeVisible()
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
		await expect(actionsBtn).toBeVisible()
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

		let column = this.getColumn(columnName)
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

	async deleteForeignKey() {
		// TODO: do not assume a single foreign key
		const fkSection = this.foreignKeySection()
		const deleteBtn = fkSection.locator('.fk-delete-btn')
		await deleteBtn.click()
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
		const lastFk = fkSection.locator('tr').nth(-2)

		await Dropdown.selectOption(this.page, lastFk.locator('input.fk-table-select'), referencedTable)
		await Dropdown.selectOption(this.page, lastFk.locator('.fk-source-col-select input'), fromCol)
		await Dropdown.selectOption(this.page, lastFk.locator('.fk-target-col-select input'), toCol)

		const fkSettings = lastFk.locator('.fk-settings-btn')
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

	async expectNoChangesDetected() {
		const btn = this.tableEditor().locator(`button:has-text("No changes detected")`)
		return await expect(btn).toBeVisible()
	}

	getColumn(columnName: string): Column {
		return new Column(this.page, this.columnsSection(), columnName)
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

	async rowOrUndefined(): Promise<Locator | undefined> {
		const rows = await this.columnsSection.locator('tr:has(input)').all()
		for (const row of rows) {
			const val = await row.locator('input').first().inputValue()
			if (val === this.columnName) return row
		}
	}
	row = async () => {
		let row = await this.rowOrUndefined()
		if (!row) {
			await this.page.waitForTimeout(500)
			row = await this.rowOrUndefined()
		}
		if (!row) throw new Error(`Column with name ${this.columnName} not found`)
		return row
	}

	primaryKeyCheckbox = async () => (await this.row()).locator('input.primary-key-checkbox')

	async setName(columnName: string) {
		const newColNameInput = (await this.row()).locator('td').nth(0).locator('input')
		await newColNameInput.fill(columnName)
		this.columnName = columnName
	}

	async checkNameIs(columnName: string) {
		const newColNameInput = (await this.row()).locator('td').nth(0).locator('input')
		await expect(newColNameInput).toHaveValue(columnName)
	}

	async setType(columnType: string) {
		await Dropdown.selectOption(
			this.page,
			(await this.row()).locator('td').nth(1).locator('input'),
			columnType
		)
	}

	async checkTypeIs(columnType: string) {
		const newColTypeSelect = (await this.row()).locator('td').nth(1).locator('input')
		await expect(newColTypeSelect).toHaveValue(new RegExp(`^${columnType}$`, 'i'))
	}

	async delete() {
		const deleteBtn = (await this.row()).locator('button.delete-column-btn')
		await deleteBtn.click()
		await this.page.waitForTimeout(50)
	}

	async setPrimaryKey(isPrimaryKey: boolean) {
		let primaryKeyCheckbox = await this.primaryKeyCheckbox()
		const isChecked = await primaryKeyCheckbox.isChecked()
		if (isChecked !== isPrimaryKey) {
			await primaryKeyCheckbox.click()
		}
	}

	async checkPrimaryKeyIs(isPrimaryKey: boolean) {
		let primaryKeyCheckbox = await this.primaryKeyCheckbox()
		const isChecked = await primaryKeyCheckbox.isChecked()
		expect(isChecked).toBe(isPrimaryKey)
	}

	async setSettings(options: { nullable?: boolean; defaultValue?: string }) {
		const settingsBtn = (await this.row()).locator('.settings-menu-btn')
		await settingsBtn.click()

		if (options.defaultValue !== undefined) {
			const defaultValueInput = this.page.locator('input.default-value')
			await defaultValueInput.fill(options.defaultValue)
		}

		if (options.nullable !== undefined) {
			const nullableCheckbox = this.page.locator('input.nullable-checkbox')
			const isChecked = await nullableCheckbox.isChecked()
			if (isChecked !== options.nullable) {
				await nullableCheckbox.click()
			}
		}
		// Close the popover
		await settingsBtn.click()
	}

	async checkSettingsIs(options: { nullable?: boolean; defaultValue?: string | RegExp }) {
		const settingsBtn = (await this.row()).locator('.settings-menu-btn')
		await settingsBtn.click()
		if (options.defaultValue !== undefined) {
			const defaultValueInput = this.page.locator('input.default-value')
			await expect(defaultValueInput).toHaveValue(options.defaultValue)
		}

		if (options.nullable !== undefined) {
			const nullableCheckbox = this.page.locator('input.nullable-checkbox')
			const isChecked = await nullableCheckbox.isChecked()
			expect(isChecked).toBe(options.nullable)
		}
		// Close the popover
		await settingsBtn.click()
	}

	async checkNotExists() {
		expect(await this.rowOrUndefined()).toBeUndefined()
	}
}

class InsertRowDrawer {
	page: Page

	constructor(page: Page) {
		this.page = page
	}

	drawer = () => this.page.locator('#insert-row-drawer')

	async fillField(fieldName: string, value: string) {
		await expect(this.drawer()).toBeVisible()
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
		await expect(cell).toBeVisible()
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

// Ensure exact casing of datatype as per DB_TYPES
function getDbDatatype(dbType: _DbType, datatype: string): string {
	if (dbType === 'ms_sql_server' && datatype.toLowerCase() === 'timestamp') datatype = 'datetime2'
	if (dbType === 'bigquery' && datatype.toLowerCase() === 'text') datatype = 'string'
	if (dbType === 'bigquery' && datatype.toLowerCase() === 'int') datatype = 'int64'
	if (dbType === 'snowflake' && datatype.toLowerCase() === 'text') datatype = 'varchar'
	const allDataTypes = DB_TYPES[dbType == 'ducklake' ? 'duckdb' : dbType] || []
	return allDataTypes.find((dt) => dt.toLowerCase() === datatype.toLowerCase()) || datatype
}

function identifier(dbType: _DbType, baseName: string): string {
	baseName = baseName.replace(/[^a-zA-Z0-9_]/g, '_').trim()
	if (dbType === 'snowflake') return baseName.toUpperCase()
	return baseName
}
