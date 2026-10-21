<script lang="ts">
	import { onMount } from 'svelte'
	import { page } from '$app/state'
	import { CONFIG_MESSAGE, KEY_MESSAGE, decodeConfig, type Config } from '../model'
	import { workspaceFor } from '../fixtures'
	import FakeHome from '../FakeHome.svelte'

	// The fake Home alone, for the playground's frame: a frame of its own so the page's
	// breakpoints answer to the width picked there. The panel posts every change here.

	const now = Date.now()

	const initial = decodeConfig(page.url.searchParams)
	let config = $state<Config>(initial)
	applyTheme(initial.dark)

	function applyTheme(dark: boolean) {
		document.documentElement.classList.toggle('dark', dark)
	}

	onMount(() => {
		function onMessage(e: MessageEvent) {
			if (e.origin !== location.origin || e.data?.type !== CONFIG_MESSAGE) return
			config = e.data.config
			applyTheme(config.dark)
		}
		// Arrow keys switch variants from the panel; forwarded so they work wherever the
		// focus is.
		function onKeydown(e: KeyboardEvent) {
			if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return
			if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return
			window.parent.postMessage({ type: KEY_MESSAGE, key: e.key }, location.origin)
		}
		window.addEventListener('message', onMessage)
		window.addEventListener('keydown', onKeydown)
		return () => {
			window.removeEventListener('message', onMessage)
			window.removeEventListener('keydown', onKeydown)
		}
	})
</script>

<svelte:head><title>Home cards frame</title></svelte:head>

{#key config.preset}
	<FakeHome {config} workspace={workspaceFor(config.preset, now)} {now} />
{/key}
