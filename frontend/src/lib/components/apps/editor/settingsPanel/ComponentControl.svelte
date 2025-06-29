<script lang="ts">
	import { ExternalLink } from 'lucide-svelte'
	import PanelSection from './common/PanelSection.svelte'
	import type { components } from '../component'
	import { getComponentControl } from '../componentsPanel/componentControlUtils'
	import { Highlight } from 'svelte-highlight'
	import typescript from 'svelte-highlight/languages/typescript'
	import { Button } from '$lib/components/common'
	import HighlightTheme from '$lib/components/HighlightTheme.svelte'
	interface Props {
		type: keyof typeof components
	}

	let { type }: Props = $props()

	const componentControls = getComponentControl(type)

	let collapsed: boolean = $state(true)
</script>

<HighlightTheme />

{#if componentControls?.length > 0}
	<PanelSection title="Controls">
		{#snippet action()}
			<div class="flex justify-end flex-wrap gap-1">
				<Button
					color="light"
					size="xs2"
					btnClasses="text-2xs font-normal"
					on:click={() => {
						collapsed = !collapsed
					}}
				>
					{collapsed ? 'Show' : 'Hide'} details
				</Button>
			</div>
		{/snippet}

		{#if collapsed}
			<div class="flex flex-row gap-1 flex-wrap">
				{#each componentControls as control}
					<span class="inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium border">
						{control.title}
					</span>
				{/each}
			</div>
		{:else}
			<div class="text-xs"
				>This component can be controlled by frontend scripts using these functions:</div
			>
			{#each componentControls as control}
				<div class="text-xs leading-6 font-semibold">
					{control.title}
				</div>
				<div class="text-xs">
					{control.description}
				</div>
				<div class="p-1 border w-full overflow-x-auto">
					<Highlight language={typescript} code={control.example} />
				</div>
				{#if control.documentation}
					<a
						href={control.documentation}
						target="_blank"
						class="text-frost-500 dark:text-frost-300 font-semibold text-xs"
					>
						<div class="flex flex-row gap-2">
							See documentation
							<ExternalLink size="16" />
						</div>
					</a>
				{/if}
			{/each}
		{/if}
	</PanelSection>
{/if}
