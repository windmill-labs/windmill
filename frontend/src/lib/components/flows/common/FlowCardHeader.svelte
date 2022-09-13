<script lang="ts">
	import IconedPath from '$lib/components/IconedPath.svelte'
	import type { FlowModule } from '$lib/gen'

	export let flowModule: FlowModule | undefined = undefined
	export let title: string | undefined = undefined

	$: flowModuleTitle =
		flowModule?.summary ||
		(flowModule?.value.type === 'rawscript'
			? `Inline ${flowModule?.value.language}`
			: 'Select a script')
</script>

<div
	class="flex items-center justify-between flex-wrap py-2 px-4 border-b bg-gray-50 shadow-sm h-12"
>
	{#if flowModule}
		<span class="text-xs font-bold text-gray-900 flex flex-col shrink">
			{#if 'path' in flowModule.value && flowModule.value.path}
				<IconedPath path={flowModule.value.path} />
			{:else if 'language' in flowModule.value && flowModule.value.language}
				{flowModuleTitle}
			{/if}
		</span>
	{/if}
	{#if title}
		<span class="text-xs font-bold text-gray-900 flex flex-col">{title}</span>
	{/if}
	<slot />
</div>
