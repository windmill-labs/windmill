<script lang="ts">
	import type { TabsContext } from '$lib/components/apps/editor/settingsPanel/inputEditor/tabs.svelte'
	import { getContext } from 'svelte'

	interface Props {
		value: string
		alwaysMounted?: boolean
		style?: string
		values?: string[] | undefined
		class?: string
		children?: import('svelte').Snippet
	}

	let {
		value,
		alwaysMounted = false,
		style = '',
		values = undefined,
		class: clazz = '',
		children
	}: Props = $props()

	const { selected } = getContext<TabsContext>('Tabs')
</script>

{#if value === $selected || alwaysMounted || values?.includes($selected)}
	<div
		class={`${clazz} ${value === $selected || values?.includes($selected) ? 'visible' : 'hidden'}`}
		{style}
	>
		{@render children?.()}
	</div>
{/if}
