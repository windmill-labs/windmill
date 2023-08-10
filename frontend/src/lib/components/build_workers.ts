import type { Environment } from 'monaco-editor/esm/vs/editor/editor.api.js'

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
