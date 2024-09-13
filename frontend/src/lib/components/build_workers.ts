import { useWorkerFactory } from 'monaco-editor-wrapper/workerFactory'

// import cssWorker from 'monaco-editor-wrapper/workers/module/css?worker&url'
// import htmlWorker from 'monaco-editor-wrapper/workers/module/html?worker&url'
// import jsonWorker from 'monaco-editor-wrapper/workers/module/json?worker&url'
// import editorWorker from 'monaco-editor-wrapper/workers/module/editor?worker&url'

export function buildWorkerDefinition(workerPath: string, basePath: string, ...args: any[]) {
	useWorkerFactory({
		ignoreMapping: true,
		workerLoaders: {
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
				const workerFilename = `graphql.worker.bundle.js`
				const workerPathLocal = `${workerPath}/${workerFilename}`
				const workerUrl = new URL(workerPathLocal, basePath)
				return new Worker(workerUrl.href, {
					name: 'graphql'
				})
			}
		}
	})
}
