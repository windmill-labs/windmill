<script lang="ts">
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'

	import Icon from 'svelte-awesome'

	export let open = false
	export let text: string
</script>

<button class="text-gray-600 underline" on:click={() => (open = !open)}>
	{text}&nbsp;<Icon data={open ? faChevronUp : faChevronDown} scale={0.5} />
</button>
{#if open}
	<div><slot /></div>
{/if}
