<script lang="ts">
	import { copyToClipboard } from '$lib/utils'
	import { Copy } from 'lucide-svelte'
	import Highlight from 'svelte-highlight'
	import type { LanguageType } from 'svelte-highlight/languages'

	interface Props {
		code: string
		language: LanguageType<string>
		disabled?: boolean
		wrap?: boolean
	}

	let { code, language, disabled = false, wrap = false }: Props = $props()
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="flex flex-col flex-1 border rounded-md relative bg-surface-input"
	class:cursor-not-allowed={disabled}
	onclick={(e) => {
		if (disabled) {
			return
		}
		e.preventDefault()
		copyToClipboard(code)
	}}
>
	<div class="absolute top-2 right-1 z-10 pointer-events-none">
		<Copy size={14} class="w-8 cursor-pointer pointer-events-auto" />
	</div>
	<div class="p-2 w-full overflow-auto">
		<Highlight
			{language}
			{code}
			class="pointer-events-none {wrap ? 'whitespace-pre-wrap break-all pr-8' : ''}"
		/>
	</div>
</div>
