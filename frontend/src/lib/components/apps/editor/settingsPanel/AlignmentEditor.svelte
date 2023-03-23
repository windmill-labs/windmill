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
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import type { AppComponent } from '../component'
	import PanelSection from './common/PanelSection.svelte'

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	export let component: AppComponent
</script>

{#if component.horizontalAlignment || component.verticalAlignment}
	<PanelSection title="Alignment">
		<div class="flex flex-wrap gap-2">
			{#if component.horizontalAlignment}
				<div class="flex flex-col gap-0.5">
					<div class="text-xs font-semibold">Horizontal</div>
					<div>
						<ToggleButtonGroup
							on:selected={() => (component = component)}
							bind:selected={component.horizontalAlignment}
						>
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
				</div>
			{/if}
			{#if component.type !== 'formcomponent' && component.verticalAlignment}
				<div class="flex flex-col gap-0.5">
					<div class="text-xs font-semibold">Vertical</div>
					<div>
						<ToggleButtonGroup
							on:selected={() => ($app = $app)}
							bind:selected={component.verticalAlignment}
						>
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
				</div>
			{/if}
		</div>
	</PanelSection>
{/if}
