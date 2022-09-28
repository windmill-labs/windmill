<script lang="ts">
	import type { BadgeColor } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import IconedPath from '$lib/components/IconedPath.svelte'
	import { RawScript, type FlowModule } from '$lib/gen'
	import { isEmptyFlowModule } from '../flowStateUtils'

	export let flowModule: FlowModule | undefined = undefined
	export let title: string | undefined = undefined

	$: shouldPick = flowModule && isEmptyFlowModule(flowModule)

	const languageColors: Record<RawScript.language, BadgeColor> = {
		[RawScript.language.GO]: 'dark-indigo',
		[RawScript.language.DENO]: 'dark-blue',
		[RawScript.language.PYTHON3]: 'dark-green'
	}
</script>

<div
	class="flex items-center justify-between py-2 px-4 border-b bg-gray-50 shadow-sm space-x-2 h-12 flex-nowrap"
>
	{#if flowModule}
		<span class="text-sm">
			<div class="flex items-center space-x-2">
				{#if shouldPick}
					<span class="font-bold text-xs">Select a script</span>
				{:else if flowModule?.value.type === 'rawscript'}
					<Badge color={languageColors[flowModule?.value.language] ?? 'gray'} capitalize>
						{flowModule?.value.language}
					</Badge>
					<input
						bind:value={flowModule.summary}
						placeholder={`Inline ${flowModule?.value.language}`}
					/>
				{:else if flowModule?.value.type === 'script' && 'path' in flowModule.value && flowModule.value.path}
					<IconedPath path={flowModule.value.path} />
					<input
						bind:value={flowModule.summary}
						placeholder={flowModule.value.path.startsWith('hub/')
							? 'Hub script'
							: 'Workspace script'}
						class="ml-2"
					/>
				{/if}
			</div>
		</span>
	{/if}
	{#if title}
		<span class="text-xs font-bold text-gray-900 flex flex-col">{title}</span>
	{/if}
	<slot />
</div>
