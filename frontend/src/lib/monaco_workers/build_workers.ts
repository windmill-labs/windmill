import { useWorkerFactory } from 'monaco-editor-wrapper/workerFactory'

// import cssWorker from 'monaco-editor-wrapper/workers/module/css?worker&url'
// import htmlWorker from 'monaco-editor-wrapper/workers/module/html?worker&url'
// import jsonWorker from 'monaco-editor-wrapper/workers/module/json?worker&url'
// import editorWorker from 'monaco-editor-wrapper/workers/module/editor'

export function buildWorkerDefinition() {
	useWorkerFactory({
		ignoreMapping: true,
		workerLoaders: {
			editorWorkerService: () => {
				return new Worker(new URL('./editorWorker-es.js', import.meta.url), {
					type: 'module'
				})
			},
			javascript: () => {
				return new Worker(new URL('./tsWorker-es.js', import.meta.url), {
					type: 'module'
				})
			},
			typescript: () => {
				return new Worker(new URL('./tsWorker-es.js', import.meta.url), {
					type: 'module'
				})
			},
			json: () => {
				return new Worker(new URL('./jsonWorker-es.js', import.meta.url), {
					type: 'module'
				})
			},
			html: () => {
				return new Worker(new URL('./htmlWorker-es.js', import.meta.url), {
					type: 'module'
				})
			},
			css: () => {
				return new Worker(new URL('./cssWorker-es.js', import.meta.url), {
					type: 'module'
				})
			},
			graphql: () => {
				// const workerFilename =
				// const workerPathLocal = `${workerPath}/${workerFilename}`
				// const workerUrl = new URL(workerPathLocal, basePath)
				// return new Worker(workerUrl.href, {
				// 	name: 'graphql'
				// })
				return new Worker(new URL(`./graphql.worker.bundle.js`, import.meta.url), {
					name: 'graphql'
				})
			}
		}
	})
}
