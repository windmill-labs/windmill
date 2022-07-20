<script lang="ts">
	import type { IconDefinition } from '@fortawesome/free-brands-svg-icons'
	import Icon from 'svelte-awesome'

	export let disabled: boolean = false
	export let icon: IconDefinition
	export let label: string
	export let iconColor: string
</script>

<button {disabled} type="button" on:click class="default-secondary-button-v2 mb-2 w-full">
	<Icon data={icon} class={`w-4 h-4 mr-2 -ml-2 ${iconColor}`} />
	{label}
</button>
