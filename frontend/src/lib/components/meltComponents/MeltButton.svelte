<script lang="ts">
	import { type AnyMeltElement } from '@melt-ui/svelte'
	import { conditionalMelt } from '$lib/utils'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import type { Snippet } from 'svelte'

	interface Props {
		aiId?: string | undefined
		aiDescription?: string | undefined
		meltElement?: AnyMeltElement | undefined
		type?: 'button' | 'submit' | 'reset' | null | undefined
		title?: string
		id?: string | undefined
		class?: string
		onclick?: (e: MouseEvent) => void
		children?: Snippet
	}

	let {
		aiId = undefined,
		aiDescription = undefined,
		meltElement = undefined,
		type = undefined,
		title = '',
		id = undefined,
		class: className = undefined,
		onclick: onclickProp = undefined,
		children
	}: Props = $props()

	let buttonRef: HTMLButtonElement | undefined = $state(undefined)
</script>

<button
	bind:this={buttonRef}
	use:conditionalMelt={meltElement}
	use:triggerableByAI={{
		id: aiId,
		description: aiDescription,
		callback: () => {
			buttonRef?.click()
		}
	}}
	class={className}
	{type}
	{title}
	{id}
	{...$meltElement}
	onclick={onclickProp}
>
	{@render children?.()}
</button>
