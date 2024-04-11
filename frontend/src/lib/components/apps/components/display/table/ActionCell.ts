import type { ICellRendererComp, ICellRendererParams } from 'ag-grid-community'
import AppAggridTableActions from './AppAggridTableActions.svelte'
import type { TableAction } from '$lib/components/apps/editor/component'
import type { AppViewerContext } from '$lib/components/apps/types'
import type { SvelteComponent } from 'svelte'

interface ActionCellParams extends ICellRendererParams {
	props: {
		id: string
		actions: TableAction[]
		render: boolean
	}
	context: AppViewerContext
}

export class ActionCell implements ICellRendererComp {
	private eGui!: HTMLDivElement
	private appActionsInstance?: SvelteComponent
	private previousId?: string
	private previousActions?: TableAction[]

	init(params: ActionCellParams) {
		console.log(params.props.actions, this.previousActions)
		if (this.shouldComponentUpdate(params.props.id, params.props.actions)) {
			this.eGui = document.createElement('div')
			this.eGui.classList.add('flex', 'flex-row', 'items-center', 'w-full', 'h-full')
			this.previousId = params.props.id
			this.previousActions = params.props.actions

			// If an instance already exists, destroy it before creating a new one
			if (this.appActionsInstance) {
				this.appActionsInstance.$destroy()
			}

			this.appActionsInstance = new AppAggridTableActions({
				target: this.eGui,
				props: {
					id: params.props.id,
					actions: params.props.actions,
					render: params.props.render,
					row: params.data,
					rowIndex: params.node.rowIndex!
				},
				context: new Map([['AppViewerContext', params.context]])
			})
		}
	}

	getGui() {
		return this.eGui
	}

	refresh(params: ActionCellParams): boolean {
		// Optionally, implement logic here to update the component
		// without fully reinitializing it if possible
		return this.shouldComponentUpdate(params.props.id, params.props.actions)
	}

	private shouldComponentUpdate(newId: string, newActions: TableAction[]): boolean {
		return (
			this.previousId !== newId ||
			JSON.stringify(this.previousActions) !== JSON.stringify(newActions)
		)
	}
}
