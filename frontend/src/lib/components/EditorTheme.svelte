<script lang="ts">
	import { getEditorTheme, isInitialized } from './vscode'

	import { editor as meditor } from 'monaco-editor'

	import { onMount } from 'svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'

	function onThemeChange() {
		if (isInitialized) {
			meditor.setTheme(getEditorTheme())
		}
	}

	onMount(() => {
		onThemeChange()
	})
</script>

<DarkModeObserver on:change={onThemeChange} />
