import { useWorkerFactory } from 'monaco-editor-wrapper/workerFactory'
import cssWorker from 'monaco-editor-wrapper/workers/module/css?worker'
import htmlWorker from 'monaco-editor-wrapper/workers/module/html?worker'
import tsWorker from 'monaco-editor-wrapper/workers/module/ts?worker'
import jsonWorker from 'monaco-editor-wrapper/workers/module/json?worker'
import editorWorker from 'monaco-editor-wrapper/workers/module/editor?worker'

export function buildWorkerDefinition(workerPath: string, basePath: string, ...args: any[]) {
	useWorkerFactory({
		ignoreMapping: true,
		workerLoaders: {
			editorWorkerService: () => {
				console.log('editorWorkerService')
				return new editorWorker()
			},
			javascript: () => {
				console.log('javascript')
				return new tsWorker()
			},

			typescript: () => {
				console.log('typescript')
				return new tsWorker()
			},
			json: () => {
				console.log('json')
				return new jsonWorker()
			},
			html: () => {
				console.log('html')
				return new htmlWorker()
			},
			css: () => {
				console.log('html')
				return new cssWorker()
			},
			graphql: () => {
				console.log('graphql')
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
