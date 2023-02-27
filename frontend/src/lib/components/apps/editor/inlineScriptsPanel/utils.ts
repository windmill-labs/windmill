import type { Schema } from '$lib/common'
import type { AppInputs, Runnable } from '../../inputType'
import type { GridItem } from '../../types'
import { fieldTypeToTsType, schemaToInputsSpec } from '../../utils'
import type { AppComponent } from '../component'

export interface AppScriptsList {
	inline: { name: string; id: string }[]
	imported: { name: string; id: string }[]
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
				newInput.subFieldType !== oldInput.subFieldType
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
	const componentInput = component.componentInput
	if (component.type === 'tablecomponent') {
		component.actionButtons.forEach((actionButton) => {
			if (actionButton.componentInput?.type !== 'runnable') {
				return
			}
			processRunnable(actionButton.componentInput.runnable, actionButton.id, list)
		})
	}
	if (componentInput?.type === 'runnable') {
		processRunnable(componentInput.runnable, gridItem.id, list)
	}
	return list
}

export function getAppScripts(
	lazyGrid: GridItem[],
	subgrids: Record<string, GridItem[]> | undefined
): AppScriptsList {
	const scriptsList = lazyGrid.reduce(
		(acc, gridComponent) => processGridItemRunnable(gridComponent, acc),
		{ inline: [], imported: [] } as AppScriptsList
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

function processRunnable(runnable: Runnable, id: string, list: AppScriptsList) {
	if (runnable?.type === undefined) {
		return
	}
	const type: keyof AppScriptsList = runnable.type === 'runnableByPath' ? 'imported' : 'inline'
	list[type].push({
		name: runnable[runnable.type === 'runnableByPath' ? 'path' : 'name'],
		id
	})
}
