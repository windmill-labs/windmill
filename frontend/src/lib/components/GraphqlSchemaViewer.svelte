<script lang="ts">
	import { BROWSER } from 'esm-env'

	import { editor as meditor } from 'monaco-editor'

	import { onDestroy, onMount } from 'svelte'

	let divEl: HTMLDivElement | null = null
	let editor: meditor.IStandaloneCodeEditor

	export let code: string = ''

	async function loadMonaco() {
		editor = meditor.create(divEl as HTMLDivElement, {
			value: code,
			language: 'graphql',
			readOnly: true,
			automaticLayout: true,
			scrollBeyondLastLine: false,
			lineNumbers: 'off',
			minimap: { enabled: false }
		})
	}

	onMount(async () => {
		if (BROWSER) {
			await loadMonaco()
		}
	})

	onDestroy(() => {
		try {
			editor && editor.dispose()
		} catch (err) {}
	})
</script>

<div bind:this={divEl} class="{$$props.class ?? ''} editor"></div>
