<script lang="ts">
	import { WindmillIcon } from './icons'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { Building } from 'lucide-svelte'
	interface Props {
		selected?: 'hub' | 'workspace'
	}

	let { selected = $bindable('hub') }: Props = $props()
</script>

<div class="max-w-min">
	<ToggleButtonGroup bind:selected>
		{#snippet children({ item })}
			<ToggleButton value="hub" icon={WindmillIcon} label="Hub" {item} />
			<ToggleButton value="workspace" icon={Building} label="Workspace" {item} />
		{/snippet}
	</ToggleButtonGroup>
</div>
