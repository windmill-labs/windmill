<script lang="ts">
	import { versionRangeToVersion } from '$lib/ata'
	import Editor from '$lib/components/Editor.svelte'
	import { extToLang } from '$lib/editorUtils'
	import {
		loadSandpackClient,
		type BundlerState,
		type SandpackClient
	} from '@codesandbox/sandpack-client'
	import { onMount } from 'svelte'
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
		"react": "^18",
		"react-dom": "^18"
	}
}`
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
						react: '^18',
						'react-dom': '^18'
					}
				},
				{
					// bundlerURL: 'http://localhost:3001/',
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

	$: activeFile && onActiveFileChange()
</script>

<button
	on:click={() => {
		activeFile = '/index.css'
	}}>CSS</button
>
<button
	on:click={() => {
		activeFile = '/index.tsx'
	}}>TSX</button
>
<button
	on:click={() => {
		activeFile = '/package.json'
	}}>PACKAGE</button
>

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
			onContentChange()
		}}
	/>
	<iframe class="min-h-screen w-full" bind:this={iframe} />
</div>

<style>
</style>
