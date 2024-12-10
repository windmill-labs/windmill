<script>
	import { loadSandpackClient } from '@codesandbox/sandpack-client'
	import { onMount } from 'svelte'

	onMount(async () => {
		const client = await loadSandpackClient('#container', {
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
		// code here
	})
</script>

<main>
	<div id="container" />
</main>

<style>
	.container {
		width: 900px;
	}
</style>
