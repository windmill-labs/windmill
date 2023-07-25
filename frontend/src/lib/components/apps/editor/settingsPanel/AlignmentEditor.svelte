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
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	export let component: AppComponent
</script>

{#if component.horizontalAlignment || component.verticalAlignment}
	<div class="flex flex-wrap gap-x-4 gap-y-1 w-full justify-end items-center">
		<div class="text-tertiary text-xs">Alignment</div>
		{#if component.horizontalAlignment}
			<ToggleButtonGroup
				noWFull
				on:selected={() => (component = component)}
				bind:selected={component.horizontalAlignment}
			>
				<ToggleButton value="left" icon={AlignStartVertical} />
				<ToggleButton value="center" icon={AlignCenterVertical} />
				<ToggleButton value="right" icon={AlignEndVertical} />
			</ToggleButtonGroup>
		{/if}
		{#if component.type !== 'formcomponent' && component.verticalAlignment}
			<ToggleButtonGroup
				noWFull
				on:selected={() => ($app = $app)}
				bind:selected={component.verticalAlignment}
			>
				<ToggleButton value="top" icon={AlignStartHorizontal} />
				<ToggleButton value="center" icon={AlignCenterHorizontal} />
				<ToggleButton value="bottom" icon={AlignEndHorizontal} />
			</ToggleButtonGroup>
		{/if}
	</div>
{/if}
