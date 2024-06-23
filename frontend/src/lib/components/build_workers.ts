import type { Environment } from 'monaco-editor/esm/vs/editor/editor.api.js'

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

interface MonacoEnvironmentEnhanced extends Environment {
	workerOverrideGlobals: WorkerOverrideGlobals
}

type WorkerOverrideGlobals = {
	basePath: string
	workerPath: string
	workerOptions: WorkerOptions
}

export function buildWorkerDefinition(
	workerPath: string,
	basePath: string,
	useModuleWorker: boolean
) {
	const monWin = self as Window
	const workerOverrideGlobals: WorkerOverrideGlobals = {
		basePath: basePath,
		workerPath: workerPath,
		workerOptions: {
			type: useModuleWorker ? 'module' : 'classic'
		}
	}

	if (!monWin.MonacoEnvironment) {
		monWin.MonacoEnvironment = {
			workerOverrideGlobals: workerOverrideGlobals,
			createTrustedTypesPolicy: (_policyName: string) => {
				return undefined
			}
		} as MonacoEnvironmentEnhanced
	}
	const monEnv = monWin.MonacoEnvironment as MonacoEnvironmentEnhanced
	monEnv.workerOverrideGlobals = workerOverrideGlobals

	const getWorker = (_: string, label: string) => {
		const buildWorker = (
			globals: WorkerOverrideGlobals,
			label: string,
			workerName: string,
			editorType: string
		) => {
			globals.workerOptions.name = label

			const workerFilename =
				globals.workerOptions.type === 'module' ? `${workerName}-es.js` : `${workerName}-iife.js`
			const workerPathLocal = `${globals.workerPath}/${workerFilename}`
			const workerUrl = new URL(workerPathLocal, globals.basePath)
			console.log(
				`${editorType}: url: ${workerUrl.href} created from basePath: ${globals.basePath} and file: ${workerPathLocal}`
			)

			return new Worker(workerUrl.href, globals.workerOptions)
		}

		console.log('getWorker: workerId: ' + _ + ' label: ' + label)

		switch (label) {
			case 'template':
			case 'typescript':
			case 'javascript':
				return buildWorker(workerOverrideGlobals, label, 'tsWorker', 'TS Worker')
			case 'html':
			case 'handlebars':
			case 'razor':
				return buildWorker(workerOverrideGlobals, label, 'htmlWorker', 'HTML Worker')
			case 'css':
			case 'scss':
			case 'less':
				return buildWorker(workerOverrideGlobals, label, 'cssWorker', 'CSS Worker')
			case 'json':
				return buildWorker(workerOverrideGlobals, label, 'jsonWorker', 'JSON Worker')
			case 'graphql':
				const workerFilename = `graphql.worker.bundle.js`
				const workerPathLocal = `${workerOverrideGlobals.workerPath}/${workerFilename}`
				const workerUrl = new URL(workerPathLocal, workerOverrideGlobals.basePath)
				return new Worker(workerUrl.href, {
					name: label
				})
			default:
				return buildWorker(workerOverrideGlobals, label, 'editorWorker', 'Editor Worker')
		}
	}

	monWin.MonacoEnvironment.getWorker = getWorker
}
