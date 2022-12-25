<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import {
		AlignCenterHorizontal,
		AlignCenterVertical,
		AlignEndHorizontal,
		AlignEndVertical,
		AlignStartHorizontal,
		AlignStartVertical
	} from 'lucide-svelte'
	import type { AppComponent } from '../../types'
	import PanelSection from './common/PanelSection.svelte'

	export let component: AppComponent
</script>

{#if component.verticalAlignment !== undefined}
	<PanelSection title="Alignment">
		{#if component.horizontalAlignment}
			<div class="w-full text-xs font-semibold">Horizontal</div>

			<div class="w-full">
				<ToggleButtonGroup bind:selected={component.horizontalAlignment}>
					<ToggleButton position="left" value="left" size="xs">
						<AlignStartVertical size={16} />
					</ToggleButton>
					<ToggleButton position="center" value="center" size="xs">
						<AlignCenterVertical size={16} />
					</ToggleButton>
					<ToggleButton position="right" value="right" size="xs">
						<AlignEndVertical size={16} />
					</ToggleButton>
				</ToggleButtonGroup>
			</div>
		{/if}
		{#if component.type !== 'formcomponent' && component.verticalAlignment}
			<div class="w-full text-xs font-semibold">Vertical</div>
			<div class="w-full">
				<ToggleButtonGroup bind:selected={component.verticalAlignment}>
					<ToggleButton position="left" value="top" size="xs">
						<AlignStartHorizontal size={16} />
					</ToggleButton>
					<ToggleButton position="center" value="center" size="xs">
						<AlignCenterHorizontal size={16} />
					</ToggleButton>
					<ToggleButton position="right" value="bottom" size="xs">
						<AlignEndHorizontal size={16} />
					</ToggleButton>
				</ToggleButtonGroup>
			</div>
		{/if}
	</PanelSection>
{/if}
