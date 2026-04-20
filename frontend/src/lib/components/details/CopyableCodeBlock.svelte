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
		copyOnClick?: boolean
	}

	let { code, language, disabled = false, wrap = false, copyOnClick = true }: Props = $props()

	function copyCode(event?: MouseEvent) {
		if (disabled) {
			return
		}
		event?.preventDefault()
		event?.stopPropagation()
		copyToClipboard(code)
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="flex flex-col flex-1 border rounded-md relative bg-surface-input"
	class:cursor-not-allowed={disabled}
	class:cursor-pointer={copyOnClick && !disabled}
	class:cursor-text={!copyOnClick && !disabled}
	onclick={(e) => {
		if (copyOnClick) {
			copyCode(e)
		}
	}}
>
	<div class="absolute top-2 right-1 z-10">
		<div class="w-8 cursor-pointer" onclick={copyCode}>
			<Copy size={14} />
		</div>
	</div>
	<div class="p-2 w-full overflow-auto">
		<Highlight
			{language}
			{code}
			class="{copyOnClick ? 'pointer-events-none' : 'select-text'} {wrap ? 'whitespace-pre-wrap break-all pr-8' : ''}"
		/>
	</div>
</div>
