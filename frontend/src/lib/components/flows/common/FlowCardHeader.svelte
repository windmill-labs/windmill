<script lang="ts">
	import type { BadgeColor } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import IconedPath from '$lib/components/IconedPath.svelte'
	import { RawScript, ScriptService, type FlowModule, type PathScript } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	export let flowModule: FlowModule | undefined = undefined
	export let title: string | undefined = undefined

	const languageColors: Record<RawScript.language, BadgeColor> = {
		[RawScript.language.GO]: 'dark-indigo',
		[RawScript.language.DENO]: 'dark-blue',
		[RawScript.language.PYTHON3]: 'dark-green',
		[RawScript.language.BASH]: 'dark-yellow'
	}

	let newHash: string | undefined = undefined
	async function loadLatestHash(value: PathScript) {
		let script = await ScriptService.getScriptByPath({
			workspace: $workspaceStore!,
			path: value.path
		})
		if (script.hash != value.hash) {
			newHash = script.hash
		}
	}

	$: $workspaceStore &&
		flowModule?.value.type === 'script' &&
		flowModule.value.hash &&
		loadLatestHash(flowModule.value)
</script>

<div
	class="overflow-x-auto scrollbar-hidden flex items-center justify-between py-2 px-4 border-b border-gray-300 space-x-2  h-full max-h-12 flex-nowrap"
>
	{#if flowModule}
		<span class="text-sm w-full mr-4">
			<div class="flex items-center space-x-2">
				{#if flowModule.value.type === 'identity'}
					<span class="font-bold text-xs">Identity (input copied to output)</span>
				{:else if flowModule?.value.type === 'rawscript'}
					<Badge color={languageColors[flowModule?.value.language] ?? 'gray'} capitalize>
						{flowModule?.value.language}
					</Badge>
					<input bind:value={flowModule.summary} placeholder={'Summary'} class="w-full grow" />
				{:else if flowModule?.value.type === 'script' && 'path' in flowModule.value && flowModule.value.path}
					<IconedPath path={flowModule.value.path} hash={flowModule.value.hash} class="grow" />
					{#if newHash}
						<Button
							size="xs"
							variant="border"
							on:click={() => {
								if (flowModule?.value.type == 'script') {
									flowModule.value.hash = newHash
									newHash = undefined
								}
							}}>Update to latest hash</Button
						>
					{/if}
					<input bind:value={flowModule.summary} placeholder="Summary" class="w-full grow" />
				{:else if flowModule?.value.type === 'flow'}
					<Badge color="indigo" capitalize>flow</Badge>
					<input bind:value={flowModule.summary} placeholder="Summary" class="w-full grow" />
				{/if}
			</div>
		</span>
	{/if}
	{#if title}
		<div class="text-sm font-bold text-gray-900">{title}</div>
	{/if}
	<slot />
</div>
