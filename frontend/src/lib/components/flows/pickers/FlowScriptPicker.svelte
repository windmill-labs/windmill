<script lang="ts">
	import type { IconDefinition } from '@fortawesome/free-brands-svg-icons'
	import { Button } from 'flowbite-svelte'
	import Icon from 'svelte-awesome'

	export let disabled: boolean = false
	export let icon: IconDefinition
	export let label: string
	export let iconColor: string
</script>

<Button {disabled} on:click size="md" class="text-left" color="alternative">
	<Icon data={icon} class={`mr-2 ${iconColor} w-4`} scale={1.5} />
	<span class="text-sm text-left">{label}</span>
</Button>
