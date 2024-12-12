import { useWorkerFactory } from 'monaco-editor-wrapper/workerFactory'

export function buildWorkerDefinition(logger: any) {
	useWorkerFactory({
		workerOverrides: {
			ignoreMapping: true,
			workerLoaders: {
				editorWorkerService: () => {
					return new Worker(
						new URL('monaco-editor-wrapper/workers/module/editor', import.meta.url),
						{
							type: 'module'
						}
					)
				},
				TextEditorWorker: () =>
					new Worker(new URL('monaco-editor/esm/vs/editor/editor.worker.js', import.meta.url), {
						type: 'module'
					}),
				TextMateWorker: () =>
					new Worker(
						new URL('@codingame/monaco-vscode-textmate-service-override/worker', import.meta.url),
						{ type: 'module' }
					),
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
		},
		logger
	})
}
