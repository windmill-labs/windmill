<script lang="ts">
	import { faBuilding } from '@fortawesome/free-solid-svg-icons'
	import { ToggleButton, ToggleButtonGroup } from './common'
	import { WindmillIcon } from './icons'
	export let selected: 'hub' | 'workspace' = 'hub'
</script>

<div class="max-w-min">
	<ToggleButtonGroup bind:selected>
		<ToggleButton position="left" value="hub" size="sm">
			<WindmillIcon white={selected == 'hub'} height="16px" width="16px" /><div class="mr-1" />Hub
		</ToggleButton>
		<ToggleButton position="right" value="workspace" size="sm" startIcon={{ icon: faBuilding }}>
			Workspace
		</ToggleButton>
	</ToggleButtonGroup>
</div>
