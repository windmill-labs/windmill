<script lang="ts">
	import { WindmillIcon2 } from './icons'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { Building, Plug } from 'lucide-svelte'
	interface Props {
		selected?: 'all' | 'hub' | 'workspace' | 'mcp'
		showMcp?: boolean
	}

	let { selected = $bindable('all'), showMcp = false }: Props = $props()
</script>

<div class="max-w-min">
	<ToggleButtonGroup id="flow-editor-flow-providers" bind:selected>
		{#snippet children({ item })}
			<ToggleButton value="all" label="All" light {item} />
			<ToggleButton value="hub" icon={WindmillIcon2} label="Hub" light {item} />
			<ToggleButton value="workspace" icon={Building} label="Workspace" light {item} />
			{#if showMcp}
				<ToggleButton value="mcp" icon={Plug} label="MCP" light {item} />
			{/if}
		{/snippet}
	</ToggleButtonGroup>
</div>
