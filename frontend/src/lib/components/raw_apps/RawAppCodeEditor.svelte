<script lang="ts">
	import '@codingame/monaco-vscode-javascript-default-extension'
	import '@codingame/monaco-vscode-css-default-extension'
	import '@codingame/monaco-vscode-json-default-extension'
	import '@codingame/monaco-vscode-html-default-extension'
	import '@codingame/monaco-vscode-standalone-languages'

	// import '@codingame/monaco-vscode-standalone-typescript-language-features'

	import '@codingame/monaco-vscode-typescript-basics-default-extension'
	import '@codingame/monaco-vscode-typescript-language-features-default-extension'

	import { initLocaleLoader } from 'monaco-editor-wrapper/vscode/locale'

	import { onDestroy, onMount } from 'svelte'
	import { Loader2 } from 'lucide-svelte'
	import { runApplicationPlayground } from './vscode'
	import { MonacoEditorLanguageClientWrapper } from 'monaco-editor-wrapper'
	import { IFileService, StandaloneServices } from 'vscode/services'
	import '@codingame/monaco-vscode-standalone-css-language-features'

	import { initFile } from '@codingame/monaco-vscode-files-service-override'
	import { Uri } from 'vscode'
	import { VSBuffer } from 'vscode/vscode/vs/base/common/buffer'
	import type { ReadOnlyMemoryFileSystemProvider } from './readonly_filesystem'

	export let user_files: Record<string, string>
	export let node_modules: Record<string, string>
	export let wmill_ts: string

	let activityBar
	let sidebar
	let editors
	let panel

	let mounted = false
	const wrapper = new MonacoEditorLanguageClientWrapper() // @hmr:keep

	let readOnlyProvider: ReadOnlyMemoryFileSystemProvider | undefined = undefined
	onMount(async () => {
		if (!mounted) {
			if (!wrapper.isStarted()) {
				try {
					for (const [name, content] of Object.entries(user_files)) {
						await initFile(Uri.file(name), content)
					}
				} catch (err) {
					// yes we know, service was already initialized
					console.error(err)
				}
			}
			await initLocaleLoader()

			readOnlyProvider = await runApplicationPlayground(
				wrapper,
				activityBar,
				sidebar,
				editors,
				panel,
				node_modules
			)
			updateReadOnlyFile()
			let fs = StandaloneServices.get(IFileService)
			// const encoder = new TextEncoder()
			for (const [name, content] of Object.entries(user_files)) {
				await fs?.writeFile(Uri.file(name), VSBuffer.fromString(content), {})
			}

			mounted = true
		}
	})

	$: readOnlyProvider && node_modules && updateReadOnlyFile()

	function updateReadOnlyFile() {
		// console.log(node_modules)
		readOnlyProvider?.rebuildTree(node_modules)
	}
	// fs
	//foop
	onDestroy(async () => {
		try {
			await wrapper.dispose()
			// await disposable?.()
		} catch (err) {
			console.error(err)
		}
	})
</script>

<div class="h-full w-full relative">
	{#if !mounted}
		<div class="h-full w-full absolute top-0 left-0 bg-surface-secondary center-center z-20">
			<div class="flex gap-2">
				Loading editor <Loader2 class="animate-spin" />
			</div>
		</div>
	{/if}
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
	/* 
	#activityBar-vscode {
		width: 0px;
		display: none;
	} */

	.codicon-accounts-view-bar-icon {
		visibility: hidden;
	}

	.codicon-settings-view-bar-icon {
		visibility: hidden;
	}

	.title-label > h2 {
		display: none;
	}
</style>
