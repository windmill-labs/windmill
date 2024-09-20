<script lang="ts">
	import { WindmillIcon } from './icons'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { Building } from 'lucide-svelte'
	export let selected: 'all' | 'hub' | 'workspace' = 'all'
</script>

<div class="max-w-min">
	<ToggleButtonGroup bind:selected>
		<ToggleButton value="all" label="All" />
		<ToggleButton value="hub" icon={WindmillIcon} label="Hub" />
		<ToggleButton value="workspace" icon={Building} label="Workspace" />
	</ToggleButtonGroup>
</div>
