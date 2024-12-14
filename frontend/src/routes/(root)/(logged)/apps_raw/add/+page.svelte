<script lang="ts">
	import Editor from '$lib/components/Editor.svelte'
	import {
		loadSandpackClient,
		type BundlerState,
		type SandpackClient
	} from '@codesandbox/sandpack-client'
	import { onMount } from 'svelte'
	let editor: Editor | undefined = undefined

	let iframe: HTMLIFrameElement | undefined = undefined

	let code: string = `import React from 'react';
import { createRoot } from 'react-dom/client';

const App = () => {
  return <div style={{ width: "100%" }}><h1>Hello, Wooorldd!</h1>
    <div style={{ width: "100%", height: "100%", background: "red" }}>BAR</div></div>;
};

const root = createRoot(document.getElementById('root')!);
root.render(<App />);`

	function buildFiles(code: string) {
		return {
			'/index.tsx': {
				code
			},
			'/index.css': {
				code: `
body: {
	background: blue;
}`
			}
		}
	}
	let files = buildFiles(code)

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
						uuid: 'latest',
						react: '^18',
						'react-dom': '^18'
					}
				},
				{
					bundlerURL: 'http://localhost:3001/',
					showOpenInCodeSandbox: false
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
</script>

<div class="w-full grid grid-cols-2">
	<Editor
		lang="typescript"
		scriptLang="tsx"
		bind:this={editor}
		bind:code
		on:change={() => {
			files = buildFiles(code)
			client?.updateSandbox({ files })
		}}
	/>
	<iframe class="min-h-screen w-full" bind:this={iframe} />
</div>

<style>
</style>
