<script lang="ts">
	import TableSimple from '$lib/components/TableSimple.svelte'
	import { listThemes, type Theme, createTheme } from './themeUtils'
	import { workspaceStore } from '$lib/stores'
	import { onMount } from 'svelte'
	import Path from '$lib/components/Path.svelte'

	export let cssString: string | undefined = undefined

	let path: string = ''
	let themes: Theme[] = []

	async function getThemes() {
		themes = await listThemes($workspaceStore!)

		console.log(themes.map((x) => x))
	}

	async function addTheme() {
		const theme: Theme = {
			path: path,
			value: cssString
		}

		await createTheme($workspaceStore!, theme)
	}

	onMount(() => {
		getThemes()
	})
</script>

<div class="p-2">
	<Path bind:path initialPath="" namePlaceholder={'theme'} kind="theme" />
	<button on:click={() => addTheme()}>add</button>

	{#if Array.isArray(themes) && themes.length > 0}
		<TableSimple size="sm" headers={['Path', 'value']} data={themes} keys={['path', 'value']} />
	{/if}
</div>
