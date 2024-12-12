<script lang="ts">
	import * as vscode from 'vscode'
	import { LogLevel } from 'vscode/services'
	import { type RegisterLocalProcessExtensionResult } from 'vscode/extensions'
	import getConfigurationServiceOverride, {
		type IStoredWorkspace
	} from '@codingame/monaco-vscode-configuration-service-override'
	import getKeybindingsServiceOverride from '@codingame/monaco-vscode-keybindings-service-override'
	import getLifecycleServiceOverride from '@codingame/monaco-vscode-lifecycle-service-override'
	import getExplorerServiceOverride from '@codingame/monaco-vscode-explorer-service-override'
	import getSearchServiceOverride from '@codingame/monaco-vscode-search-service-override'
	import {
		RegisteredFileSystemProvider,
		registerFileSystemOverlay,
		RegisteredMemoryFile
	} from '@codingame/monaco-vscode-files-service-override'
	import { attachPart, type Parts } from '@codingame/monaco-vscode-views-service-override'

	// import '@codingame/monaco-vscode-json-default-extension'
	// import '@codingame/monaco-vscode-standalone-json-language-features'
	import '@codingame/monaco-vscode-typescript-basics-default-extension'
	import '@codingame/monaco-vscode-typescript-language-features-default-extension'
	import { useWorkerFactory } from 'monaco-editor-wrapper/workerFactory'
	import { initLocaleLoader } from 'monaco-editor-wrapper/vscode/locale'
	import { MonacoEditorLanguageClientWrapper, type WrapperConfig } from 'monaco-editor-wrapper'
	// import { useWorkerFactory } from 'monaco-editor-wrapper/workerFactory'
	import { onMount } from 'svelte'

	export const configureMonacoWorkers = (logger?: any) => {
		useWorkerFactory({
			workerOverrides: {
				ignoreMapping: true,
				workerLoaders: {
					TextEditorWorker: () =>
						new Worker(new URL('monaco-editor/esm/vs/editor/editor.worker.js', import.meta.url), {
							type: 'module'
						}),
					TextMateWorker: () =>
						new Worker(
							new URL('@codingame/monaco-vscode-textmate-service-override/worker', import.meta.url),
							{ type: 'module' }
						)
				}
			},
			logger
		})
	}

	export const createDefaultWorkspaceFile = (workspaceFile: vscode.Uri, workspacePath: string) => {
		return new RegisteredMemoryFile(
			workspaceFile,
			JSON.stringify(
				<IStoredWorkspace>{
					folders: [
						{
							path: workspacePath
						}
					]
				},
				null,
				2
			)
		)
	}

	const wrapper = new MonacoEditorLanguageClientWrapper()

	const runApplicationPlayground = async () => {
		const workspaceFile = vscode.Uri.file('/workspace/.vscode/workspace.code-workspace')

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
				console.log(config.part)
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
					...getExplorerServiceOverride(),
					// ...getEnvironmentServiceOverride(),
					...getSearchServiceOverride()

					// ...getSearchServiceOverride(),
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
						'oct.serverUrl': 'https://api.open-collab.tools/',
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

		const helloTsUri = vscode.Uri.file('/workspace/hello.ts')
		const testerTsUri = vscode.Uri.file('/workspace/tester.ts')
		const fileSystemProvider = new RegisteredFileSystemProvider(false)
		fileSystemProvider.registerFile(new RegisteredMemoryFile(helloTsUri, `async function foo() {}`))
		fileSystemProvider.registerFile(
			new RegisteredMemoryFile(testerTsUri, `async function foo() {}`)
		)
		fileSystemProvider.registerFile(createDefaultWorkspaceFile(workspaceFile, '/workspace'))
		registerFileSystemOverlay(1, fileSystemProvider)
		fileSystemProvider.registerFile(
			new RegisteredMemoryFile(
				vscode.Uri.file('/workspace/types/function.d.ts'),
				'declare function foo(): 42'
			)
		)

		await wrapper.init(wrapperConfig)
		const result = wrapper.getExtensionRegisterResult(
			'mlc-app-playground'
		) as RegisterLocalProcessExtensionResult
		result.setAsDefaultApi()

		await Promise.all([await vscode.window.showTextDocument(helloTsUri)])
		mounted = true
	}

	let mounted = false
	onMount(async () => {
		if (!mounted) {
			await initLocaleLoader()

			runApplicationPlayground()
			console.log('rerun')
		}
	})

	let activityBar
	let sidebar
	// let auxiliaryBar
	let editors
	let panel
</script>

<div id="workbench-container-vscode">
	<div id="workbench-top-vscode">
		<div id="sidebarDiv-vscode">
			<div id="activityBar-vscode" bind:this={activityBar} />
			<div id="sidebar-vscode" bind:this={sidebar} />
			<!-- <div id="auxiliaryBar" bind:this={auxiliaryBar} /> -->
		</div>
		<div id="editorsDiv-vscode">
			<div id="editors-vscode" bind:this={editors} />
		</div>
	</div>
	<div id="panel-vscode" bind:this={panel} />
</div>

<style global>
	.workbench-container-vscode {
		font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
		font-size: 16px;
		line-height: 24px;
		font-weight: 400;

		font-synthesis: none;
		text-rendering: optimizeLegibility;
		-webkit-font-smoothing: antialiased;
		-moz-osx-font-smoothing: grayscale;
		-webkit-text-size-adjust: 100%;
	}

	/* body {
    background-color: var(--vscode-editorWidget-background);
    color: var(--vscode-editorWidget-foreground);
    margin: 0;
  } */

	#sidebarDiv-vscode {
		display: flex;
		flex: none;
		border: 1px solid var(--vscode-editorWidget-border);
	}

	#sidebar-vscode {
		width: 300px;
	}

	#editorsDiv-vscode {
		flex: 1;
		min-width: 0;
	}

	#editors-vscode {
		position: relative;
		min-width: 0;
		height: 95%;
		border: 1px solid var(--vscode-editorWidget-border);
	}

	#auxiliaryBar-vscode {
		max-width: 300px;
	}

	#panel-vscode {
		display: none;
		flex: 1;
		border: 1px solid var(--vscode-editorWidget-border);
		min-height: 0;
	}

	#workbench-container-vscode {
		height: 95vh;
		display: flex;
		flex-direction: column;
	}

	#workbench-top-vscode {
		display: flex;
		flex: 2;
		min-height: 0;
	}

	.codicon-accounts-view-bar-icon {
		visibility: hidden;
	}

	.codicon-settings-view-bar-icon {
		visibility: hidden;
	}
</style>
