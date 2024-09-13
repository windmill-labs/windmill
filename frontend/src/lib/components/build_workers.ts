import { useWorkerFactory } from 'monaco-editor-wrapper/workerFactory'

import tsWorker from 'monaco-editor-wrapper/workers/module/ts?worker&url'
import cssWorker from 'monaco-editor-wrapper/workers/module/css?worker&url'
import htmlWorker from 'monaco-editor-wrapper/workers/module/html?worker&url'
import jsonWorker from 'monaco-editor-wrapper/workers/module/json?worker&url'
import editorWorker from 'monaco-editor-wrapper/workers/module/editor?worker&url'

export function buildWorkerDefinition(workerPath: string, basePath: string, ...args: any[]) {
	useWorkerFactory({
		ignoreMapping: true,
		workerLoaders: {
			editorWorkerService: () => {
				return new Worker(new URL(editorWorker, import.meta.url), {
					type: 'module'
				})
			},
			javascript: () => {
				return new Worker(new URL(tsWorker, import.meta.url), {
					type: 'module'
				})
			},
			typescript: () => {
				return new Worker(new URL(tsWorker, import.meta.url), {
					type: 'module'
				})
			},
			json: () => {
				return new Worker(new URL(jsonWorker, import.meta.url), {
					type: 'module'
				})
			},
			html: () => {
				return new Worker(new URL(htmlWorker, import.meta.url), {
					type: 'module'
				})
			},
			css: () => {
				return new Worker(new URL(cssWorker, import.meta.url), {
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
