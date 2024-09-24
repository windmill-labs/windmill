<script lang="ts">
	import { WindmillIcon2 } from './icons'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { Building } from 'lucide-svelte'
	export let selected: 'all' | 'hub' | 'workspace' = 'all'
</script>

<div class="max-w-min">
	<ToggleButtonGroup bind:selected>
		<ToggleButton value="all" label="All" light small />
		<ToggleButton value="hub" icon={WindmillIcon2} label="Hub" light small />
		<ToggleButton value="workspace" icon={Building} label="Workspace" light small />
	</ToggleButtonGroup>
</div>
