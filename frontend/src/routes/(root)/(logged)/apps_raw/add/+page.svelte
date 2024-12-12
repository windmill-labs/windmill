<script lang="ts">
	import RawAppEditor from '$lib/components/RawAppEditor.svelte'
	import { loadSandpackClient, type SandpackClient } from '@codesandbox/sandpack-client'
	import { onMount } from 'svelte'

	let iframe: HTMLIFrameElement | undefined = undefined

	let code: string = `
import React from 'react';
import ReactDOM from 'react-dom';

const App = () => {
    return <div style={{width: "100%" }}><h1>Hello, Worldddd!</h1>
	<div style={{width: "100%", height: "100%", background: "red"}}>BAR</div></div>;
};

ReactDOM.render(<App />, document.getElementById('root'));
`
	let files: Record<string, { code: string }> = {
		'/index.jsx': {
			code
		}
	}

	let client: SandpackClient | undefined = undefined
	onMount(async () => {
		if (iframe) {
			client = await loadSandpackClient(
				iframe,
				{
					files,
					entry: '/index.jsx',
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
				console.log('Message from iframe:', msg)
			})
		}
		// code here
	})
</script>

<RawAppEditor />

<!-- <iframe class="min-h-screen w-full" bind:this={iframe} /> -->

<style>
</style>
