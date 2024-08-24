// inline to make it a library component easy to be imported until this if fixed: https://github.com/vitejs/vite/pull/16418
// import cssWorker from 'monaco-editor/esm/vs/language/css/css.worker?worker&inline'
// import htmlWorker from 'monaco-editor/esm/vs/language/html/html.worker?worker&inline'
// import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker&inline'
// import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker&inline'
// import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker&inline'

// import cssWorker from 'monaco-editor/esm/vs/language/css/css.worker?worker'
// import htmlWorker from 'monaco-editor/esm/vs/language/html/html.worker?worker'
// import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker'
// import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker'
// import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'

// import type { Environment } from 'monaco-editor/esm/vs/editor/editor.api.js'

// interface MonacoEnvironmentEnhanced extends Environment {
// 	workerOverrideGlobals: WorkerOverrideGlobals
// }

// type WorkerOverrideGlobals = {
// 	basePath: string
// 	workerPath: string
// 	workerOptions: WorkerOptions
// }

// export function buildWorkerDefinition(
// 	workerPath: string,
// 	basePath: string,
// 	useModuleWorker: boolean
// ) {
// 	const monWin = self as Window
// 	const workerOverrideGlobals: WorkerOverrideGlobals = {
// 		basePath: basePath,
// 		workerPath: workerPath,
// 		workerOptions: {
// 			type: useModuleWorker ? 'module' : 'classic'
// 		}
// 	}

// 	if (!monWin.MonacoEnvironment) {
// 		monWin.MonacoEnvironment = {
// 			workerOverrideGlobals: workerOverrideGlobals,
// 			createTrustedTypesPolicy: (_policyName: string) => {
// 				return undefined
// 			}
// 		} as MonacoEnvironmentEnhanced
// 	}
// 	const monEnv = monWin.MonacoEnvironment as MonacoEnvironmentEnhanced
// 	monEnv.workerOverrideGlobals = workerOverrideGlobals

// 	const getWorker = (_: string, label: string) => {
// 		console.log('getWorker: workerId: ' + _ + ' label: ' + label)

// 		switch (label) {
// 			case 'template':
// 			case 'typescript':
// 			case 'javascript':
// 				return new tsWorker()
// 			case 'html':
// 			case 'handlebars':
// 			case 'razor':
// 				return new htmlWorker()
// 			case 'css':
// 			case 'scss':
// 			case 'less':
// 				return new cssWorker()
// 			case 'json':
// 				return new jsonWorker()
// 			case 'graphql':
// 				const workerFilename = `graphql.worker.bundle.js`
// 				const workerPathLocal = `${workerOverrideGlobals.workerPath}/${workerFilename}`
// 				const workerUrl = new URL(workerPathLocal, workerOverrideGlobals.basePath)
// 				return new Worker(workerUrl.href, {
// 					name: label
// 				})
// 			default:
// 				return new editorWorker()
// 		}
// 	}

// 	monWin.MonacoEnvironment.getWorker = getWorker
// }

// //
// export type WorkerLoader = () => Worker

// const workerLoaders: Partial<Record<string, WorkerLoader>> = {
// 	editorWorkerService: () =>
// 		new Worker(new URL('monaco-editor/esm/vs/editor/editor.worker.js', import.meta.url), {
// 			type: 'module'
// 		}),
// 	textMateWorker: () =>
// 		new Worker(
// 			new URL('@codingame/monaco-vscode-textmate-service-override/worker', import.meta.url),
// 			{ type: 'module' }
// 		),
// 	languageDetectionWorkerService: () =>
// 		new Worker(
// 			new URL(
// 				'@codingame/monaco-vscode-language-detection-worker-service-override/worker',
// 				import.meta.url
// 			),
// 			{ type: 'module' }
// 		)
// }
// export function registerWorkerLoader(label: string, workerLoader: WorkerLoader): void {
// 	workerLoaders[label] = workerLoader
// }

// export function buildWorkerDefinition() {
// 	// Do not use monaco-editor-webpack-plugin because it doesn't handle properly cross origin workers
// 	window.MonacoEnvironment = {
// 		getWorker: function (moduleId, label) {
// 			console.log('LOAD')
// 			const workerFactory = workerLoaders[label]
// 			if (workerFactory != null) {
// 				return workerFactory()
// 			}
// 			throw new Error(`Unimplemented worker ${label} (${moduleId})`)
// 		}
// 	}
// }

import { graphql } from 'graphql'
import { useWorkerFactory } from 'monaco-editor-wrapper/workerFactory'

export function buildWorkerDefinition(...args: any[]) {
	useWorkerFactory({
		ignoreMapping: true,
		workerLoaders: {
			editorWorkerService: () => {
				console.log('editorWorkerService')
				return new Worker(
					new URL('monaco-editor/esm/vs/editor/editor.worker.js', import.meta.url),
					{
						type: 'module'
					}
				)
			},
			javascript: () => {
				console.log('javascript')
				return new Worker(new URL('monaco-editor-wrapper/workers/module/ts', import.meta.url), {
					type: 'module'
				})
			},

			typescript: () => {
				console.log('typescript')
				return new Worker(new URL('monaco-editor-wrapper/workers/module/ts', import.meta.url), {
					type: 'module'
				})
			},
			json: () => {
				console.log('json')
				return new Worker(new URL('monaco-editor-wrapper/workers/module/json', import.meta.url), {
					type: 'module'
				})
			},
			html: () => {
				console.log('html')
				return new Worker(new URL('monaco-editor-wrapper/workers/module/html', import.meta.url), {
					type: 'module'
				})
			},
			css: () => {
				console.log('html')
				return new Worker(new URL('monaco-editor-wrapper/workers/module/css', import.meta.url), {
					type: 'module'
				})
			}
		}
	})
}
