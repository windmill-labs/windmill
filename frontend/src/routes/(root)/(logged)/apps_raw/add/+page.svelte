<script lang="ts">
	import { loadSandpackClient } from '@codesandbox/sandpack-client'
	import { onMount } from 'svelte'

	let iframe: HTMLIFrameElement | undefined = undefined
	onMount(async () => {
		if (iframe) {
			const client = await loadSandpackClient(iframe, {
				files: {
					'/index.js': {
						code: `console.log(require('uuid'))`
					}
				},
				entry: '/index.js',
				dependencies: {
					uuid: 'latest'
				}
			})
		}
		// code here
	})
</script>

<main>
	<iframe bind:this={iframe} />
</main>

<style>
	.container {
		width: 900px;
	}
</style>
