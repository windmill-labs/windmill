<script lang="ts">
	import {
		AlignCenterHorizontal,
		AlignCenterVertical,
		AlignEndHorizontal,
		AlignEndVertical,
		AlignStartHorizontal,
		AlignStartVertical
	} from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import type { AppComponent } from '../component'
	import PanelSection from './common/PanelSection.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	export let component: AppComponent
</script>

{#if component.horizontalAlignment || component.verticalAlignment}
	<PanelSection title="Alignment">
		<div class="flex flex-col gap-2 w-full">
			{#if component.horizontalAlignment}
				<div class="flex flex-row justify-between items-center">
					<div class="text-xs font-semibold">Horizontal</div>
					<div>
						<ToggleButtonGroup
							on:selected={() => (component = component)}
							bind:selected={component.horizontalAlignment}
						>
							<ToggleButton value="left" icon={AlignStartVertical} />
							<ToggleButton value="center" icon={AlignCenterVertical} />
							<ToggleButton value="right" icon={AlignEndVertical} />
						</ToggleButtonGroup>
					</div>
				</div>
			{/if}
			{#if component.type !== 'formcomponent' && component.verticalAlignment}
				<div class="flex flex-row justify-between items-center">
					<div class="text-xs font-semibold">Vertical</div>
					<div>
						<ToggleButtonGroup
							on:selected={() => ($app = $app)}
							bind:selected={component.verticalAlignment}
						>
							<ToggleButton value="top" icon={AlignStartHorizontal} />
							<ToggleButton value="center" icon={AlignCenterHorizontal} />
							<ToggleButton value="bottom" icon={AlignEndHorizontal} />
						</ToggleButtonGroup>
					</div>
				</div>
			{/if}
		</div>
	</PanelSection>
{/if}
