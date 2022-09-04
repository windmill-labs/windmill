<script lang="ts">
	import { getContext } from 'svelte'
	import type { TabsContext } from './Tabs.svelte'

	export let value: string
	let clazz: string = ''
	export { clazz as class }
	const { selected } = getContext<TabsContext>('Tabs')
</script>

{#if value === $selected}
	<div class={clazz}>
		<slot />
	</div>
{/if}
