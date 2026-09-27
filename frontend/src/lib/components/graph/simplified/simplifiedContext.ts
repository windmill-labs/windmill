import { getContext, setContext } from 'svelte'
import { GitBranchPlus } from 'lucide-svelte'
import type { FlowModule } from '$lib/gen'
import type { Item } from '$lib/utils'
import type { ContextMenuItem } from '../../common/contextmenu/ContextMenu.svelte'
import type { GraphEventHandlers } from '../graphBuilder.svelte'
import { getGraphContext } from '../graphContext'
import { getNoteEditorContext } from '../noteEditor.svelte'
import { buildModuleMenuItems, toContextMenuItems } from '../moduleMenuItems'

/** What the simplified view's renderers need from the editor hosting it */
export type SimplifiedEditContext = {
	/** Absent outside the editor, where nothing is editable */
	readonly eventHandlers: GraphEventHandlers | undefined
	readonly editMode: boolean
	readonly disableAi: boolean
}

const KEY = 'SimplifiedFlowGraphEdit'

export function setSimplifiedEditContext(ctx: SimplifiedEditContext): void {
	setContext(KEY, ctx)
}

export function getSimplifiedEditContext(): SimplifiedEditContext | undefined {
	return getContext<SimplifiedEditContext | undefined>(KEY)
}

/**
 * The editor's right-click menu for a step, resolved against the contexts of the component
 * calling it (so call it during component init). Empty outside edit mode.
 */
export function useStepMenu(): (id: string, module: FlowModule | undefined) => ContextMenuItem[] {
	const edit = getSimplifiedEditContext()
	const noteEditor = getNoteEditorContext()?.noteEditor
	const selectionManager = getGraphContext()?.selectionManager
	const stepExploreHint = getContext<(() => boolean) | undefined>('flowGraphStepExploreHint')
	return (id, module) => {
		const eventHandlers = edit?.eventHandlers
		if (!edit?.editMode || !eventHandlers) return []
		const type = module?.value.type
		const items: Item[] = [
			...(type === 'branchone' || type === 'branchall'
				? [
						{
							displayName: 'Add branch',
							icon: GitBranchPlus,
							action: () => eventHandlers.newBranch(id)
						}
					]
				: []),
			...buildModuleMenuItems({ id, eventHandlers, noteEditor, selectionManager, stepExploreHint })
		]
		return toContextMenuItems(items)
	}
}
