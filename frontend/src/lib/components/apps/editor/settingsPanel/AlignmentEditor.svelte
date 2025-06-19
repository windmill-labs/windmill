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

	interface Props {
		component: AppComponent
	}

	let { component = $bindable() }: Props = $props()
</script>

{#if component.horizontalAlignment || component.verticalAlignment}
	<div class="flex flex-wrap gap-x-4 gap-y-1 w-full justify-end items-center">
		<div class="text-tertiary text-xs">Alignment</div>
		{#if component.horizontalAlignment}
			<ToggleButtonGroup
				noWFull
				on:selected={() => (app.val = app.val)}
				bind:selected={component.horizontalAlignment}
			>
				{#snippet children({ item })}
					<ToggleButton value="left" icon={AlignStartVertical} {item} />
					<ToggleButton value="center" icon={AlignCenterVertical} {item} />
					<ToggleButton value="right" icon={AlignEndVertical} {item} />
				{/snippet}
			</ToggleButtonGroup>
		{/if}
		{#if component.type !== 'formcomponent' && component.verticalAlignment}
			<ToggleButtonGroup
				noWFull
				on:selected={() => (app.val = app.val)}
				bind:selected={component.verticalAlignment}
			>
				{#snippet children({ item })}
					<ToggleButton value="top" icon={AlignStartHorizontal} {item} />
					<ToggleButton value="center" icon={AlignCenterHorizontal} {item} />
					<ToggleButton value="bottom" icon={AlignEndHorizontal} {item} />
				{/snippet}
			</ToggleButtonGroup>
		{/if}
	</div>
{/if}
