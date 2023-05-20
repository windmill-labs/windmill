<script lang="ts">
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'

	import Icon from 'svelte-awesome'
	import { Button } from './common'
	import { slide } from 'svelte/transition'

	export let open = false
	export let text: string
	export let small = false
</script>

<div class="flex">
	<Button
		variant="border"
		color="light"
		btnClasses="text-gray-600 {small ? 'text-xs' : ''} "
		on:click={() => (open = !open)}
	>
		{text}&nbsp;<Icon data={open ? faChevronUp : faChevronDown} scale={0.5} />
	</Button>
</div>
{#if open}
	<div transition:slide|local={{ duration: 100 }}><slot /></div>
{/if}
