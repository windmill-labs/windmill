import type { SupportedLanguage } from '$lib/common'

export type WhitelabelCustomUi = {
	topBar?: {
		path?: boolean
		export?: boolean
		history?: boolean
		aiBuilder?: boolean
		tutorials?: boolean
		diff?: boolean
	}
	settingsPanel?: boolean
	triggers?: boolean
	flowNode?: boolean
	hub?: boolean
	graph?: { aiBuilder?: boolean; dataflow?: boolean }
	stepInputs?: { ai?: boolean }
	stepAdvancedSettings?: boolean
	languages?: (SupportedLanguage | 'docker' | 'bunnative')[]
	scriptFork?: boolean
}
