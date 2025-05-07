<script lang="ts">
	import { WindmillIcon } from './icons'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { Building } from 'lucide-svelte'
	export let selected: 'hub' | 'workspace' = 'hub'
</script>

<div class="max-w-min">
	<ToggleButtonGroup bind:selected let:item>
		<ToggleButton value="hub" icon={WindmillIcon} label="Hub" {item} />
		<ToggleButton value="workspace" icon={Building} label="Workspace" {item} />
	</ToggleButtonGroup>
</div>
