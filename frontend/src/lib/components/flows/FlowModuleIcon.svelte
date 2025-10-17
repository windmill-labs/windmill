<script lang="ts">
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import type { FlowModule } from '$lib/gen'
	import { Building, Repeat, Square, ArrowDown, GitBranch, Bot } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

	interface Props {
		module: FlowModule
		size?: number
		width?: number
		height?: number
	}

	let { module, size = 16, width, height }: Props = $props()

	// Use width/height if provided, otherwise use size for both
	const iconWidth = width || size
	const iconHeight = height || size
</script>

{#if module?.value?.type === 'aiagent'}
	<Bot size={16} class="text-ai" />
{:else if module?.value?.type === 'rawscript'}
	<LanguageIcon lang={module.value.language} width={iconWidth} height={iconHeight} />
{:else if module?.summary === 'Terminate flow'}
	<Square {size} />
{:else if module?.value?.type === 'identity'}
	<ArrowDown {size} />
{:else if module?.value?.type === 'flow'}
	<BarsStaggered {size} />
{:else if module?.value?.type === 'forloopflow' || module?.value?.type === 'whileloopflow'}
	<Repeat {size} />
{:else if module?.value?.type === 'branchone' || module?.value?.type === 'branchall'}
	<GitBranch {size} />
{:else if module?.value?.type === 'script'}
	{#if module?.value?.path?.startsWith('hub/')}
		<IconedResourceType
			width={iconWidth.toString() + 'px'}
			height={iconHeight.toString() + 'px'}
			name={module?.value?.path?.split('/')[2]}
			silent={true}
		/>
	{:else}
		<Building {size} />
	{/if}
{:else}
	<!-- Fallback icon for unknown module types -->
	<BarsStaggered {size} />
{/if}
