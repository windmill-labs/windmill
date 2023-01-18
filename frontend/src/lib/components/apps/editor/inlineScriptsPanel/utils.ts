import type { Runnable } from "../../inputType"
import type { AppComponent, GridItem } from "../../types"

export interface AppScriptsList {
	inline: { name: string; id: string }[],
	imported: { name: string; id: string }[]
}

export function getAppScripts(grid: GridItem[]) {
	return grid.reduce((acc, gridComponent) => {
		const component: AppComponent = gridComponent.data
		const componentInput = component.componentInput

		if (component.type === 'tablecomponent') {
			component.actionButtons.forEach((actionButton) => {
				if (actionButton.componentInput?.type !== 'runnable') { return }
				processRunnable(
					actionButton.componentInput.runnable,
					actionButton.id,
					acc
				)
			})
		}
		if (componentInput?.type === 'runnable') {
			processRunnable(
				componentInput.runnable,
				gridComponent.id,
				acc
			)
		}

		return acc
	}, {inline: [], imported: []} as AppScriptsList)
}

function processRunnable(runnable: Runnable, id: string, list: AppScriptsList) {
	if(runnable?.type === undefined) { return }
	const type: keyof AppScriptsList = runnable.type === 'runnableByPath' ? 'imported' : 'inline'
	list[type].push({
		name: runnable[runnable.type === 'runnableByPath' ? 'path' : 'name'],
		id
	})
}