<script lang="ts">
	import { isInitialized } from './vscode'

	import { editor as meditor } from 'monaco-editor'

	import { onMount } from 'svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'

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
