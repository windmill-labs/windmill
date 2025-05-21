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
	class="flex flex-col flex-1 border rounded-md relative"
	class:cursor-not-allowed={disabled}
	on:click={(e) => {
		if (disabled) {
			return
		}
		e.preventDefault()
		copyToClipboard(code)
	}}
>
	<div class="absolute top-2 right-1 z-10 pointer-events-none">
		<Clipboard size={14} class="w-8 cursor-pointer pointer-events-auto" />
	</div>
	<div class="p-2 overflow-auto w-full">
		<Highlight {language} {code} class="pointer-events-none" />
	</div>
</div>
