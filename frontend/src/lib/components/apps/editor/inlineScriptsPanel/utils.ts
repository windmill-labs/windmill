import type { Schema } from '$lib/common'
import { ScriptService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import type { AppInputs, Runnable, RunnableByName } from '../../inputType'
import type { GridItem, HiddenRunnable, InlineScript } from '../../types'
import { fieldTypeToTsType, schemaToInputsSpec } from '../../utils'
import type { AppComponent } from '../component'

export interface AppScriptsList {
	inline: { name: string; id: string; transformer: boolean }[]
	imported: { name: string; id: string; transformer: boolean }[]
}

// When the schema is loaded, we need to update the inputs spec
// in order to render the inputs the component panel
export function computeFields(schema: Schema, defaultUserInput: boolean, fields: AppInputs) {
	let schemaCopy: Schema = JSON.parse(JSON.stringify(schema))

	const result = {}
	const newInputs = schemaToInputsSpec(schemaCopy, defaultUserInput)
	if (!fields) {
		return newInputs
	}
	Object.keys(newInputs).forEach((key) => {
		const newInput = newInputs[key]
		const oldInput = fields[key]

		// If the input is not present in the old inputs, add it
		if (oldInput === undefined) {
			result[key] = newInput
		} else {
			if (
				fieldTypeToTsType(newInput.fieldType) !== fieldTypeToTsType(oldInput.fieldType) ||
				newInput.format !== oldInput.format ||
				newInput.subFieldType !== oldInput.subFieldType ||
				// An input became a select
				(newInput.fieldType === 'select' && oldInput.fieldType !== 'select') ||
				// The input was already a select, but the values changed
				(newInput.fieldType === 'select' &&
					oldInput.fieldType === 'select' &&
					JSON.stringify(newInput.selectOptions) !== JSON.stringify(oldInput.selectOptions))
			) {
				result[key] = newInput
			} else {
				result[key] = oldInput
			}
		}
	})

	return result
}

function processGridItemRunnable(gridItem: GridItem, list: AppScriptsList): AppScriptsList {
	const component: AppComponent = gridItem.data
	if (component) {
		const componentInput = component.componentInput
		if (component.type === 'tablecomponent') {
			component.actionButtons.forEach((actionButton) => {
				if (actionButton.componentInput?.type !== 'runnable') {
					return
				}
				processRunnable(
					actionButton.componentInput.runnable,
					actionButton.componentInput.transformer,
					actionButton.id,
					list
				)
			})
		}

		if (
			component.type === 'aggridcomponent' ||
			component.type === 'aggridcomponentee' ||
			component.type === 'dbexplorercomponent' ||
			component.type === 'aggridinfinitecomponent' ||
			component.type === 'aggridinfinitecomponentee'
		) {
			component.actions?.forEach((actionButton) => {
				if (actionButton.componentInput?.type !== 'runnable') {
					return
				}
				processRunnable(
					actionButton.componentInput.runnable,
					actionButton.componentInput.transformer,
					actionButton.id,
					list
				)
			})
		}

		if (component.type === 'menucomponent') {
			component.menuItems?.forEach((menuItem) => {
				if (menuItem.componentInput?.type !== 'runnable') {
					return
				}
				processRunnable(
					menuItem.componentInput.runnable,
					menuItem.componentInput.transformer,
					menuItem.id,
					list
				)
			})
		}

		if (componentInput?.type === 'runnable') {
			processRunnable(componentInput.runnable, componentInput.transformer, gridItem.id, list)
		}
	}
	return list
}

export function getAppScripts(
	grid: GridItem[],
	subgrids: Record<string, GridItem[]> | undefined
): AppScriptsList {
	const scriptsList = (grid ?? []).reduce(
		(acc, gridComponent) => processGridItemRunnable(gridComponent, acc),
		{ inline: [], imported: [], transformer: false } as AppScriptsList
	)

	if (subgrids) {
		Object.values(subgrids).forEach((subgrid: GridItem[]) => {
			subgrid.forEach((subgridComponent: GridItem) => {
				processGridItemRunnable(subgridComponent, scriptsList)
			})
		})
	}

	return scriptsList
}

function processRunnable(
	runnable: Runnable,
	transformer: InlineScript | undefined,
	id: string,
	list: AppScriptsList
) {
	if (runnable?.type === undefined) {
		return
	}
	const type: keyof AppScriptsList = runnable.type === 'runnableByPath' ? 'imported' : 'inline'
	list[type].push({
		name: runnable[runnable.type === 'runnableByPath' ? 'path' : 'name'],
		id,
		transformer: transformer !== undefined
	})
}

export async function createScriptFromInlineScript(
	id: string,
	runnable: HiddenRunnable | RunnableByName,
	workspace: string,
	appPath: string
) {
	if (runnable.type != 'runnableByName') {
		sendUserToast('Only inline scripts can be saved to workspace', true)
		return
	}
	if (!runnable.inlineScript) {
		sendUserToast('No inline script found', true)
		return
	}
	let language = runnable.inlineScript.language
	if (language == 'frontend') {
		sendUserToast('Frontend scripts can not be saved to workspace', true)
		return
	}
	await ScriptService.createScript({
		workspace,
		requestBody: {
			path: appPath + '/' + id,
			summary: runnable.name ?? '',
			description: '',
			content: runnable.inlineScript.content,
			parent_hash: undefined,
			schema: runnable.inlineScript.schema,
			is_template: false,
			language: language!
		}
	})

	Object.assign(runnable, {
		type: 'runnableByPath',
		schema: runnable.inlineScript.schema,
		runType: 'script',
		recomputeIds: undefined,
		path: appPath + '/' + id
	})
}
