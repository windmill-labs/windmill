<script lang="ts">
	import { editor as meditor } from 'monaco-editor/esm/vs/editor/editor.api'

	import { onMount } from 'svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { isInitialized } from './vscode'

	function onThemeChange() {
		if (isInitialized) {
			if (document.documentElement.classList.contains('dark')) {
				meditor.setTheme('nord')
			} else {
				meditor.setTheme('myTheme')
			}
		}
	}

	onMount(() => {
		onThemeChange()
	})
</script>

<DarkModeObserver on:change={onThemeChange} />
