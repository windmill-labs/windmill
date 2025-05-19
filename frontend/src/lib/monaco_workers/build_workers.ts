import { getEnhancedMonacoEnvironment } from 'monaco-languageclient/vscode/services'

// import cssWorker from 'monaco-editor-wrapper/workers/module/css?worker&url'
// import htmlWorker from 'monaco-editor-wrapper/workers/module/html?worker&url'
// import jsonWorker from 'monaco-editor-wrapper/workers/module/json?worker&url'
// import editorWorker from 'monaco-editor-wrapper/workers/module/editor?worker&url'

export function buildWorkerDefinition() {
	const envEnhanced = getEnhancedMonacoEnvironment()

	const getWorker = (moduleId: string, label: string) => {
		console.log(`getWorker: moduleId: ${moduleId} label: ${label}`)

		let selector = label
		let workerLoaders = {
			editorWorkerService: () => {
				return new Worker(new URL('monaco-editor-wrapper/workers/module/editor', import.meta.url), {
					type: 'module'
				})
			},
			javascript: () => {
				return new Worker(new URL('monaco-editor-wrapper/workers/module/ts', import.meta.url), {
					type: 'module'
				})
			},
			typescript: () => {
				return new Worker(new URL('monaco-editor-wrapper/workers/module/ts', import.meta.url), {
					type: 'module'
				})
			},
			json: () => {
				return new Worker(new URL('monaco-editor-wrapper/workers/module/json', import.meta.url), {
					type: 'module'
				})
			},
			html: () => {
				return new Worker(new URL('monaco-editor-wrapper/workers/module/html', import.meta.url), {
					type: 'module'
				})
			},
			css: () => {
				return new Worker(new URL('monaco-editor-wrapper/workers/module/css', import.meta.url), {
					type: 'module'
				})
			},
			graphql: () => {
				console.log('Creating graphql worker')
				return new Worker(new URL(`./graphql.worker.bundle.js`, import.meta.url), {
					name: 'graphql'
				})
			}
		}
		const workerFunc = workerLoaders[selector]
		if (workerFunc !== undefined) {
			return workerFunc()
		} else {
			throw new Error(`Unimplemented worker ${label} (${moduleId})`)
		}
	}
	envEnhanced.getWorker = getWorker
}
