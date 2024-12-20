<script lang="ts">
	import { versionRangeToVersion } from '$lib/ata'
	import { animateTo } from '$lib/components/apps/editor/appUtils'
	import InlineScriptsPanel from '$lib/components/apps/editor/inlineScriptsPanel/InlineScriptsPanel.svelte'
	import type { Runnable } from '$lib/components/apps/inputType'
	import { Button } from '$lib/components/common'
	import CustomPopover from '$lib/components/CustomPopover.svelte'
	import Editor from '$lib/components/Editor.svelte'
	import { extToLang } from '$lib/editorUtils'
	import {
		loadSandpackClient,
		type BundlerState,
		type SandpackClient
	} from '@codesandbox/sandpack-client'
	import { onMount, setContext } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { writable } from 'svelte/store'
	let editor: Editor | undefined = undefined

	let iframe: HTMLIFrameElement | undefined = undefined

	let activeFile = '/index.tsx'

	let files: Record<string, { code: string }> = {
		'/index.tsx': {
			code: `import React from 'react';
import { createRoot } from 'react-dom/client';

const App = () => {
  return <div style={{ width: "100%" }}><h1>Hello, Wooorldd!</h1>
    <div style={{ width: "100%", height: "100%", background: "red" }}>BAR</div></div>;
};

const root = createRoot(document.getElementById('root')!);
root.render(<App />);`
		},
		'/index.css': {
			code: `
body {
	background: blue;
}`
		},
		'/package.json': {
			code: `{
	"dependencies": {
		"react": "18.3.1",
		"react-dom": "18.3.1"
	}
}`
		},
		'/policy.json': {
			code: 'foo'
		}
	}

	let client: SandpackClient | undefined = undefined
	let bundlerState: BundlerState | undefined = undefined
	onMount(async () => {
		if (iframe) {
			client = await loadSandpackClient(
				iframe,
				{
					files,
					entry: '/index.tsx',
					dependencies: {
						react: '18.3.1',
						'react-dom': '18.3.1'
					}
				},
				{
					bundlerURL: 'http://localhost:3001/',
					showOpenInCodeSandbox: false,
					customNpmRegistries: [
						{
							limitToScopes: false,
							registryUrl: 'http://localhost:4873/',
							enabledScopes: ['*'],
							proxyEnabled: false
						}
					]
				}
			)
			client.listen((msg) => {
				if (msg.type == 'state') {
					bundlerState = msg.state
					// const libs = Object.entries(bundlerState.transpiledModules)
					// 	.filter(([name, mod]) => mod.module.path.startsWith('/node_modules'))
					// 	.map(([name, mod]) => ({
					// 		filePath: 'file://' + mod.module.path,
					// 		content: mod.module.code
					// 	}))
					// console.log('libs', libs)
					// editor?.setTypescriptExtraLibs(libs)
				}
				console.log('Message from sandbox', msg)
			})
		}
		// code here
	})

	function getLangOfExt(path: string) {
		if (path.endsWith('.tsx')) return 'typescript'
		if (path.endsWith('.ts')) return 'typescript'
		if (path.endsWith('.js')) return 'javascript'
		if (path.endsWith('.jsx')) return 'typescript'
		if (path.endsWith('.css')) return 'css'
		if (path.endsWith('.json')) return 'json'
		return 'text'
	}

	function onPackageJsonChange() {
		let pkg = JSON.parse(files['/package.json'].code)
		let dependencies: Record<string, string> =
			typeof pkg.dependencies == 'object' ? pkg?.dependencies : {}
		client?.updateSandbox({
			files,
			dependencies
		})
		const ataDeps = Object.entries(dependencies).map(([name, version]) => ({
			raw: name,
			module: name,
			version: versionRangeToVersion(version)
		}))
		editor?.fetchPackageDeps(ataDeps)
		console.log(ataDeps)
	}

	function onContentChange() {
		if (activeFile == '/package.json') {
			onPackageJsonChange()
			return
		}
		client?.updateSandbox({
			files
		})
	}

	function onActiveFileChange() {
		editor?.switchToFile(
			activeFile,
			files[activeFile].code,
			extToLang(activeFile.split('.').pop()!)
		)
	}

	let timeout: NodeJS.Timeout | undefined = undefined
	let nameInput
	let name = ''

	$: activeFile && onActiveFileChange()

	function createNewFile(newFileName: string) {
		newFileName = '/' + newFileName
		console.log('bar', newFileName)
		files = { ...files, [newFileName]: { code: '' } }
		activeFile = newFileName
	}

	let appPanelSize = 70

	function hideBottomPanel(animate: boolean = false) {
		appPanelSize = 100
	}

	setContext('AppEditorContext', {
		selectedComponentInEditor: writable(undefined)
	})

	setContext('AppViewerContext', {
		app: writable({
			hiddenInlineScripts: [] as Runnable[]
		})
	})
</script>

<div class="h-screen">
	<Splitpanes id="o2" horizontal class="!overflow-visible">
		<Pane bind:size={appPanelSize} class="ovisible">
			<div class="flex">
				<div class="flex text-xs max-w-32 flex-col gap-2 py-0.5 text-secondary">
					<CustomPopover class="text-left ml-1" appearTimeout={0} focusEl={nameInput}>
						<button class="text-left hover:text-primary">+new</button>
						<svelte:fragment slot="overlay">
							<div class="flex flex-row gap-2 min-w-72">
								<input
									type="text"
									placeholder="New file name"
									bind:value={name}
									bind:this={nameInput}
									on:keydown={(e) => {
										if (e.key === 'Enter') {
											createNewFile(name)
										}
									}}
								/>
								<Button size="xs" on:click={() => createNewFile(name)}>New</Button>
							</div>
						</svelte:fragment>
					</CustomPopover>
					{#each Object.keys(files) as file}
						<button
							class="hover:text-primary text-left truncate {activeFile == file
								? 'font-bold text-primary'
								: ''} hover:underline rounded px-1"
							on:click={() => {
								activeFile = file
							}}>{file.substring(1)}</button
						>
					{/each}
				</div>

				<div class="w-full grid grid-cols-2">
					<Editor
						lang={getLangOfExt(activeFile)}
						path={activeFile}
						scriptLang="tsx"
						bind:this={editor}
						bind:code={files[activeFile].code}
						{files}
						on:ataReady={() => {
							onPackageJsonChange()
						}}
						on:change={() => {
							timeout && clearTimeout(timeout)
							timeout = setTimeout(() => {
								onContentChange()
							}, 500)
						}}
					/>
					<iframe class="min-h-screen w-full" bind:this={iframe} />
				</div>
			</div>
		</Pane>
		<Pane>
			<!-- svelte-ignore a11y-no-static-element-interactions -->
			<div class="flex h-full w-full overflow-x-visible">
				<InlineScriptsPanel on:hidePanel={() => hideBottomPanel(true)} rawApps on:hidePanel />
			</div>
			<!-- <div class="bg-red-400 h-full w-full" /> -->
		</Pane>
	</Splitpanes>
</div>

<style>
</style>
