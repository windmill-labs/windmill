import type { Environment } from 'monaco-editor/esm/vs/editor/editor.api.js'
import cssWorker from 'monaco-editor/esm/vs/language/css/css.worker?worker'
import htmlWorker from 'monaco-editor/esm/vs/language/html/html.worker?worker'
import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker'
import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker'
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'

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
		console.log('getWorker: workerId: ' + _ + ' label: ' + label)

		switch (label) {
			case 'template':
			case 'typescript':
			case 'javascript':
				return new tsWorker()
			case 'html':
			case 'handlebars':
			case 'razor':
				return new htmlWorker()
			case 'css':
			case 'scss':
			case 'less':
				return new cssWorker()
			case 'json':
				return new jsonWorker()
			case 'graphql':
				const workerFilename = `graphql.worker.bundle.js`
				const workerPathLocal = `${workerOverrideGlobals.workerPath}/${workerFilename}`
				const workerUrl = new URL(workerPathLocal, workerOverrideGlobals.basePath)
				return new Worker(workerUrl.href, {
					name: label
				})
			default:
				return new editorWorker()
		}
	}

	monWin.MonacoEnvironment.getWorker = getWorker
}
