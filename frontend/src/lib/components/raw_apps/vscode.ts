import * as vscode from 'vscode'

import { getService, IWorkbenchLayoutService, LogLevel } from 'vscode/services'
import { type RegisterLocalProcessExtensionResult } from 'vscode/extensions'
import getConfigurationServiceOverride, {
	type IStoredWorkspace
} from '@codingame/monaco-vscode-configuration-service-override'
import getKeybindingsServiceOverride from '@codingame/monaco-vscode-keybindings-service-override'
import getLifecycleServiceOverride from '@codingame/monaco-vscode-lifecycle-service-override'
import getExplorerServiceOverride from '@codingame/monaco-vscode-explorer-service-override'
import getSearchServiceOverride from '@codingame/monaco-vscode-search-service-override'
import { useWorkerFactory } from 'monaco-editor-wrapper/workerFactory'

import {
	RegisteredFileSystemProvider,
	registerFileSystemOverlay,
	RegisteredMemoryFile,
	type IFileSystemProviderWithFileReadWriteCapability,
	FileType,
	type IFileChange,
	type IFileDeleteOptions,
	type IFileOverwriteOptions,
	type IFileWriteOptions,
	type IStat,
	type IWatchOptions,
	FileSystemProviderError,
	FileSystemProviderErrorCode
} from '@codingame/monaco-vscode-files-service-override'
import { attachPart, type Parts } from '@codingame/monaco-vscode-views-service-override'
import getMarkersServiceOverride from '@codingame/monaco-vscode-markers-service-override'
import { type WrapperConfig } from 'monaco-editor-wrapper'
import { sendUserToast } from '$lib/toast'
import { EventEmitter, Uri } from 'vscode'
import type { CancellationToken } from 'vscode/vscode/vs/base/common/cancellation'
import type { Event } from 'vscode/vscode/vs/base/common/event'
import type { IDisposable } from 'vscode/vscode/vs/base/common/lifecycle'
import type { ReadableStreamEvents } from 'vscode/vscode/vs/base/common/stream'
import type { URI } from 'vscode/vscode/vs/base/common/uri'
import type {
	IFileReadStreamOptions,
	IFileOpenOptions,
	FileSystemProviderCapabilities
} from 'vscode/vscode/vs/platform/files/common/files'
import { ReadOnlyMemoryFileSystemProvider } from './readonly_filesystem'

export const configureMonacoWorkers = (logger?: any) => {
	useWorkerFactory({
		workerOverrides: {
			ignoreMapping: true,
			workerLoaders: {
				// editorWorkerService: () => {
				// 	return new Worker(
				// 		new URL('monaco-editor-wrapper/workers/module/editor', import.meta.url),
				// 		{
				// 			type: 'module'
				// 		}
				// 	)
				// },

				TextEditorWorker: () =>
					new Worker(new URL('monaco-editor/esm/vs/editor/editor.worker.js', import.meta.url), {
						type: 'module'
					}),
				css: () => {
					return new Worker(new URL('monaco-editor-wrapper/workers/module/css', import.meta.url), {
						type: 'module'
					})
				},
				// typescript: () => {
				// 	return new Worker(new URL('monaco-editor-wrapper/workers/module/ts', import.meta.url), {
				// 		type: 'module'
				// 	})
				// },
				TextMateWorker: () =>
					new Worker(
						new URL('@codingame/monaco-vscode-textmate-service-override/worker', import.meta.url),
						{ type: 'module' }
					),
				LocalFileSearchWorker: () =>
					new Worker(
						new URL('@codingame/monaco-vscode-search-service-override/worker', import.meta.url),
						{ type: 'module' }
					)
			}
		},
		logger
	})
}

export const runApplicationPlayground = async (
	wrapper,
	activityBar,
	sidebar,
	editors,
	panel,
	node_modules
) => {
	const workspaceFile = vscode.Uri.file('/.vscode/workspace.code-workspace')

	const defaultViewsInit = () => {
		for (const config of [
			{
				part: 'workbench.parts.activitybar',
				element: activityBar
			},
			{
				part: 'workbench.parts.sidebar',
				element: sidebar
			},
			// {
			// 	part: 'workbench.parts.auxiliarybar',
			// 	element: auxiliaryBar
			// },
			{ part: 'workbench.parts.editor', element: editors },
			{ part: 'workbench.parts.panel', element: panel }
		]) {
			attachPart(config.part as Parts, config.element)
		}
	}

	const wrapperConfig: WrapperConfig = {
		$type: 'extended',
		id: 'AAP',
		logLevel: LogLevel.Debug,
		htmlContainer: document.body,
		vscodeApiConfig: {
			serviceOverrides: {
				...getConfigurationServiceOverride(),
				...getKeybindingsServiceOverride(),
				...getLifecycleServiceOverride(),
				// ...getBannerServiceOverride(),
				// ...getTitleBarServiceOverride(),
				// ...getModelServiceOverride(),
				...getSearchServiceOverride(),
				...getExplorerServiceOverride(),
				...getMarkersServiceOverride()
				// ...getEnvironmentServiceOverride(),
			},
			enableExtHostWorker: true,
			viewsConfig: {
				viewServiceType: 'ViewsService',
				viewsInitFunc: defaultViewsInit
			},
			workspaceConfig: {
				enableWorkspaceTrust: true,
				windowIndicator: {
					label: 'mlc-app-playground',
					tooltip: '',
					command: ''
				},
				workspaceProvider: {
					trusted: true,
					async open() {
						window.open(window.location.href)
						return true
					},
					workspace: {
						workspaceUri: workspaceFile
					}
				},
				configurationDefaults: {
					'window.title': 'mlc-app-playground${separator}${dirty}${activeEditorShort}'
				},
				productConfiguration: {
					nameShort: 'mlc-app-playground',
					nameLong: 'mlc-app-playground'
				}
			},
			userConfiguration: {
				json: JSON.stringify({
					'workbench.colorTheme': 'Default Dark Modern',
					'editor.wordBasedSuggestions': 'off',
					'typescript.tsserver.web.projectWideIntellisense.enabled': true,
					'typescript.tsserver.web.projectWideIntellisense.suppressSemanticErrors': false,
					'editor.guides.bracketPairsHorizontal': true,
					'editor.experimental.asyncTokenization': false
				})
			}
		},
		extensions: [
			{
				config: {
					name: 'mlc-app-playground',
					publisher: 'TypeFox',
					version: '1.0.0',
					engines: {
						vscode: '*'
					}
				}
			}
		],
		editorAppConfig: {
			monacoWorkerFactory: configureMonacoWorkers
		}
	}

	// const fileSystemProvider = new RegisteredFileSystemProvider(true)
	// fileSystemProvider.registerFile(
	// 	new RegisteredMemoryFile(Uri.file('/foo'), `async function foo() {}`)
	// )
	// // fileSystemProvider.registerFile(new RegisteredMemoryFile(testerTsUri, `async function foo() {}`))
	// fileSystemProvider.registerFile(createDefaultWorkspaceFile(workspaceFile, '/'))
	// registerFileSystemOverlay(1, fileSystemProvider)
	let staticFileSystemProvider = new ReadOnlyMemoryFileSystemProvider({
		'/.vscode/settings.json': JSON.stringify(
			{
				'files.exclude': {
					'.vscode/': true,
					node_modules: false
				}
			},
			null,
			2
		),
		'/tsconfig.json': JSON.stringify(
			{
				compilerOptions: {
					target: 'ES2020',
					lib: ['ES2020', 'DOM', 'DOM.Iterable'],
					module: 'NodeNext',
					moduleResolution: 'NodeNext',
					strict: true,
					esModuleInterop: true,
					skipLibCheck: true,
					forceConsistentCasingInFileNames: true,
					resolveJsonModule: true,
					isolatedModules: true,
					noUnusedLocals: true,
					noUnusedParameters: true,
					noImplicitReturns: true,
					noFallthroughCasesInSwitch: true,
					declaration: true,
					sourceMap: true,
					allowJs: true,
					checkJs: true
				},
				include: ['./**/*.ts', './**/*.tsx', './**/*.js', './**/*.jsx'],
				exclude: ['node_modules']
			},
			null,
			2
		),
		[workspaceFile.path]: JSON.stringify(
			{
				folders: [
					{
						path: '/'
					}
				]
			},
			null,
			2
		)
	})
	let nodeModulesfileSystemProvider = new ReadOnlyMemoryFileSystemProvider(node_modules)
	registerFileSystemOverlay(2, staticFileSystemProvider)
	registerFileSystemOverlay(1, nodeModulesfileSystemProvider)

	await wrapper.init(wrapperConfig)
	const result = wrapper.getExtensionRegisterResult(
		'mlc-app-playground'
	) as RegisterLocalProcessExtensionResult
	result.setAsDefaultApi()

	await getService(IWorkbenchLayoutService)

	// show explorer and not search
	await vscode.commands.executeCommand('workbench.view.explorer')
	sendUserToast('Welcome to the Monaco Language Client playground!')

	//necessary for hmr to work
	try {
		defaultViewsInit()
	} catch (e) {
		console.log(e)
	}

	return nodeModulesfileSystemProvider
	// await Promise.all([await vscode.window.showTextDocument(helloTsUri)])
}
