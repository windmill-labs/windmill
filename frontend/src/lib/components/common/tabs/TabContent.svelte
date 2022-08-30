<script lang="ts">
	import { getContext } from 'svelte'

	export let value: string
	let clazz: string = ''
	export { clazz as class }
	const { selected } = getContext('Tabs')
</script>

{#if value === $selected}
	<div class={clazz}>
		<slot />
	</div>
{/if}
