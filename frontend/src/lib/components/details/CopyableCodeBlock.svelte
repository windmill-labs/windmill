<script lang="ts">
	import { copyToClipboard } from '$lib/utils'
	import { Clipboard } from 'lucide-svelte'
	import Highlight from 'svelte-highlight'
	import type { LanguageType } from 'svelte-highlight/languages'

	export let code: string
	export let language: LanguageType<string>
	export let disabled = false
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class="flex flex-row flex-1 border p-2 rounded-md overflow-auto relative"
	class:cursor-not-allowed={disabled}
	on:click={(e) => {
		if (disabled) {
			return
		}
		e.preventDefault()
		copyToClipboard(code)
	}}
>
	<Highlight {language} {code} class="pointer-events-none" />
	<Clipboard size={14} class="w-8 top-2 right-2 absolute" />
</div>
