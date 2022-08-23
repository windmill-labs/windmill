<script lang="ts">
	export let value: number
	export let index: number
	let clazz: string = ''
	export { clazz as class }
</script>

{#if value === index}
	<div class={clazz}>
		<slot />
	</div>
{/if}
