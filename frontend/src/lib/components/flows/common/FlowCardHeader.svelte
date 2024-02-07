<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import MetadataGen from '$lib/components/copilot/MetadataGen.svelte'
	import IconedPath from '$lib/components/IconedPath.svelte'
	import { ScriptService, type FlowModule, type PathScript } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Lock, RefreshCw, Unlock } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	export let flowModule: FlowModule | undefined = undefined
	export let title: string | undefined = undefined

	let latestHash: string | undefined = undefined
	async function loadLatestHash(value: PathScript) {
		let script = await ScriptService.getScriptByPath({
			workspace: $workspaceStore!,
			path: value.path
		})
		latestHash = script.hash
	}

	const dispatch = createEventDispatcher()

	$: $workspaceStore &&
		flowModule?.value.type === 'script' &&
		flowModule.value.path &&
		!flowModule.value.path.startsWith('hub/') &&
		loadLatestHash(flowModule.value)
</script>

<div
	class="overflow-x-auto scrollbar-hidden flex items-center justify-between px-4 py-1 flex-nowrap"
>
	{#if flowModule}
		<span class="text-sm w-full mr-4">
			<div class="flex items-center space-x-2">
				{#if flowModule.value.type === 'identity'}
					<span class="font-bold text-xs">Identity (input copied to output)</span>
				{:else if flowModule?.value.type === 'rawscript'}
					<div class="mx-0.5">
						<LanguageIcon lang={flowModule.value.language} width={20} height={20} />
					</div>
					<MetadataGen
						bind:content={flowModule.summary}
						promptConfigName="summary"
						code={flowModule.value.content}
						class="w-full"
						elementProps={{
							placeholder: 'Summary'
						}}
					/>
				{:else if flowModule?.value.type === 'script' && 'path' in flowModule.value && flowModule.value.path}
					<IconedPath path={flowModule.value.path} hash={flowModule.value.hash} class="grow" />

					{#if flowModule.value.hash}
						{#if latestHash != flowModule.value.hash}
							<Button
								color="light"
								size="xs"
								variant="border"
								on:click={() => {
									if (flowModule?.value.type == 'script') {
										flowModule.value.hash = latestHash
									}
									dispatch('reload')
								}}>Update to latest hash</Button
							>
						{/if}
						<Button
							title="Unlock hash to always use latest deployed version at that path"
							size="xs"
							btnClasses="text-tertiary inline-flex gap-1 items-center"
							color="light"
							on:click={() => {
								if (flowModule?.value.type == 'script') {
									flowModule.value.hash = undefined
								}
							}}><Unlock size={12} />hash</Button
						>
					{:else if latestHash}
						<div class="flex">
							<Button
								title="Lock hash to always use this specific version"
								color="light"
								size="xs"
								btnClasses="text-tertiary inline-flex gap-1 items-center"
								on:click={() => {
									if (flowModule?.value.type == 'script') {
										flowModule.value.hash = latestHash
									}
								}}><Lock size={12} />hash</Button
							>
							<Button
								title="Reload latest hash"
								size="xs"
								color="light"
								on:click={() => dispatch('reload')}
							>
								<RefreshCw size={12} /></Button
							>
						</div>
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
		<div class="text-sm font-bold text-primary pr-2">{title}</div>
	{/if}
	<slot />
</div>
