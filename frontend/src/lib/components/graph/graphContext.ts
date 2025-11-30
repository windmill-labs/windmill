import { getContext, setContext } from 'svelte'
import type { SelectionManager } from './selectionUtils.svelte'
import type { NoteManager } from './noteManager.svelte'
import type { Writable } from 'svelte/store'
import type { FlowDiffManager } from '../flows/flowDiffManager.svelte'

export type GraphContext = {
	selectionManager: SelectionManager
	useDataflow: Writable<boolean | undefined>
	showAssets: Writable<boolean | undefined>
	noteManager?: NoteManager
	clearFlowSelection?: () => void
	yOffset?: number
	diffManager: FlowDiffManager
}

const graphContextKey = 'FlowGraphContext'

//TODO: use https://svelte.dev/docs/svelte/context#Type-safe-context after migrating svelte 5 to latest version
export const getGraphContext = () => getContext<GraphContext>(graphContextKey)
export const setGraphContext = (context: GraphContext) => setContext(graphContextKey, context)
