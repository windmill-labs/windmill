import { getContext, setContext } from 'svelte'
import type { SelectionManager } from './selectionUtils.svelte'
import type { Writable } from 'svelte/store'

type GraphContext = {
	selectionManager: SelectionManager
	useDataflow: Writable<boolean | undefined>
	showAssets: Writable<boolean | undefined>
}

const graphContextKey = 'FlowGraphContext'

//TODO: use https://svelte.dev/docs/svelte/context#Type-safe-context after migrating svelte 5 to latest version
export const getGraphContext = () => getContext<GraphContext>(graphContextKey)
export const setGraphContext = (context: GraphContext) => setContext(graphContextKey, context)
