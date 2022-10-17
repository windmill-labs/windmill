<script lang="ts">
	import { getContext } from 'svelte'
	import type { TabsContext } from './Tabs.svelte'

	export let value: string
	export let alwaysMounted: boolean = false
	export let style = ''

	let clazz: string = ''
	export { clazz as class }
	const { selected } = getContext<TabsContext>('Tabs')
</script>

{#if value === $selected || alwaysMounted}
	<div class={`${clazz} ${value === $selected ? 'visible' : 'hidden'}`} {style}>
		<slot />
	</div>
{/if}
