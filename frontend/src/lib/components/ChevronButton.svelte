<script lang="ts">
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'

	import Icon from 'svelte-awesome'

	export let text: string

	export let viewOptions = false
</script>

<button
	type="submit"
	class="mr-6 text-sm underline text-gray-700 inline-flex items-center"
	on:click={() => {
		viewOptions = !viewOptions
	}}
>
	<div>
		{text}
		<Icon data={viewOptions ? faChevronUp : faChevronDown} scale={0.5} />
	</div>
</button>
{#if viewOptions}
	<slot />
{/if}
