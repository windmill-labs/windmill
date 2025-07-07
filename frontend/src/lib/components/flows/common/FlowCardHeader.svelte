<script lang="ts" module>
	let cachedValues: Record<
		string,
		{
			latestHash: string | undefined
		}
	> = {}
</script>

<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import MetadataGen from '$lib/components/copilot/MetadataGen.svelte'
	import IconedPath from '$lib/components/IconedPath.svelte'
	import { ScriptService, type FlowModuleValue, type PathScript } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Lock, RefreshCw, Unlock } from 'lucide-svelte'
	import { createEventDispatcher, untrack } from 'svelte'

	interface Props {
		flowModuleValue?: FlowModuleValue | undefined
		title?: string | undefined
		summary?: string | undefined
		children?: import('svelte').Snippet
	}

	let {
		flowModuleValue = undefined,
		title = undefined,
		summary = $bindable(undefined),
		children
	}: Props = $props()

	let latestHash: string | undefined = $state(undefined)
	function getCachedKey(path: string) {
		return `${$workspaceStore}-${path}`
	}
	function getCachedValues(path: string) {
		const key = getCachedKey(path)
		latestHash = cachedValues[key]?.latestHash
	}
	if (flowModuleValue?.type === 'script' && flowModuleValue.path) {
		getCachedValues(flowModuleValue.path)
	}

	async function loadLatestHash(value: PathScript) {
		let script = await ScriptService.getScriptByPath({
			workspace: $workspaceStore!,
			path: value.path
		})
		const key = getCachedKey(value.path)
		cachedValues[key] = {
			latestHash: script.hash
		}
		latestHash = script.hash
	}

	const dispatch = createEventDispatcher()

	$effect.pre(() => {
		$workspaceStore &&
			flowModuleValue?.type === 'script' &&
			flowModuleValue.path &&
			!flowModuleValue.path.startsWith('hub/') &&
			untrack(() => loadLatestHash(flowModuleValue))
	})
</script>

<div
	class="overflow-x-auto scrollbar-hidden flex items-center justify-between px-4 py-1 flex-nowrap"
>
	{#if flowModuleValue}
		<span class="text-sm w-full mr-4">
			<div class="flex items-center space-x-2">
				{#if flowModuleValue.type === 'identity'}
					<span class="font-bold text-xs">Identity (input copied to output)</span>
				{:else if flowModuleValue.type === 'rawscript'}
					<div class="mx-0.5">
						<LanguageIcon lang={flowModuleValue.language} width={20} height={20} />
					</div>
					<MetadataGen
						bind:content={summary}
						promptConfigName="summary"
						code={flowModuleValue.content}
						class="w-full"
						elementProps={{
							placeholder: 'Summary'
						}}
					/>
				{:else if flowModuleValue.type === 'script' && 'path' in flowModuleValue && flowModuleValue.path}
					<IconedPath path={flowModuleValue.path} hash={flowModuleValue.hash} class="grow" />

					{#if flowModuleValue.hash}
						{#if latestHash != flowModuleValue.hash}
							<Button
								color="light"
								size="xs"
								variant="border"
								on:click={() => {
									if (flowModuleValue.type == 'script') {
										dispatch('setHash', latestHash)
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
								if (flowModuleValue.type == 'script') {
									dispatch('setHash', undefined)
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
									if (flowModuleValue.type == 'script') {
										dispatch('setHash', latestHash)
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
					<input bind:value={summary} placeholder="Summary" class="w-full grow" />
				{:else if flowModuleValue.type === 'flow'}
					<Badge color="indigo" capitalize>flow</Badge>
					<input bind:value={summary} placeholder="Summary" class="w-full grow" />
				{/if}
			</div>
		</span>
	{/if}
	{#if title}
		<div class="text-sm font-bold text-primary pr-2">{title}</div>
	{/if}
	{@render children?.()}
</div>
