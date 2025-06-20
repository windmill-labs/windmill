<script lang="ts">
	import { WindmillIcon2 } from './icons'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { Building } from 'lucide-svelte'
	interface Props {
		selected?: 'all' | 'hub' | 'workspace'
	}

	let { selected = $bindable('all') }: Props = $props()
</script>

<div class="max-w-min">
	<ToggleButtonGroup id="flow-editor-flow-providers" bind:selected>
		{#snippet children({ item })}
			<ToggleButton value="all" label="All" light small {item} />
			<ToggleButton value="hub" icon={WindmillIcon2} label="Hub" light small {item} />
			<ToggleButton value="workspace" icon={Building} label="Workspace" light small {item} />
		{/snippet}
	</ToggleButtonGroup>
</div>
