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
	import { hubBaseUrlStore, workspaceStore } from '$lib/stores'
	import { Flag, Lock, RefreshCw, Unlock } from 'lucide-svelte'
	import { createEventDispatcher, untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { validateToolName } from '$lib/components/graph/renderers/nodes/AIToolNode.svelte'
	import { DEFAULT_HUB_BASE_URL, PRIVATE_HUB_MIN_VERSION } from '$lib/hub'

	interface Props {
		flowModuleValue?: FlowModuleValue | undefined
		title?: string | undefined
		summary?: string | undefined
		children?: import('svelte').Snippet
		action?: import('svelte').Snippet
		isAgentTool?: boolean
	}

	let {
		flowModuleValue = undefined,
		title = undefined,
		summary = $bindable(undefined),
		children,
		action,
		isAgentTool = false
	}: Props = $props()

	let latestHash: string | undefined = $state(undefined)

	// Extract version_id from hub path (format: hub/{version_id}/{app}/{summary})
	let hubVersionId = $derived(
		flowModuleValue?.type === 'script' && flowModuleValue.path?.startsWith('hub/')
			? flowModuleValue.path.split('/')[1]
			: undefined
	)

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
	class="overflow-x-auto scrollbar-hidden flex items-center justify-between px-4 py-2 flex-nowrap"
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
						promptConfigName={isAgentTool ? 'agentToolFunctionName' : 'summary'}
						code={flowModuleValue.content}
						class="w-full"
						elementProps={{
							placeholder: isAgentTool ? 'Tool name' : 'Summary'
						}}
					/>
				{:else if flowModuleValue.type === 'script' && 'path' in flowModuleValue && flowModuleValue.path}
					<IconedPath path={flowModuleValue.path} hash={flowModuleValue.hash} class="grow" />

					{#if hubVersionId}
						<Button
							title="Report an issue with this hub script"
							unifiedSize="sm"
							variant="subtle"
							on:click={() => {
								const targetHubBaseUrl =
									Number(hubVersionId) < PRIVATE_HUB_MIN_VERSION
										? DEFAULT_HUB_BASE_URL
										: $hubBaseUrlStore
								window.open(
									`${targetHubBaseUrl}/from_version/${hubVersionId}?report_issue=${hubVersionId}`,
									'_blank'
								)
							}}
						>
							<Flag size={12} />Report issue
						</Button>
					{/if}

					{#if flowModuleValue.hash}
						{#if latestHash != flowModuleValue.hash}
							<Button
								size="xs"
								variant="default"
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
							btnClasses="text-primary inline-flex gap-1 items-center"
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
								btnClasses="text-primary inline-flex gap-1 items-center"
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
					<input
						bind:value={summary}
						placeholder={isAgentTool ? 'Tool name' : 'Summary'}
						class={twMerge(
							'w-full grow',
							isAgentTool && !validateToolName(summary ?? '') && '!border-red-400'
						)}
					/>
				{:else if flowModuleValue.type === 'flow'}
					<Badge color="indigo" capitalize>flow</Badge>
					<input bind:value={summary} placeholder="Summary" class="w-full grow" />
				{:else if flowModuleValue.type === 'aiagent'}
					<Badge color="indigo">AI Agent</Badge>
					<input bind:value={summary} placeholder="Summary" class="w-full grow" />
				{/if}
			</div>
		</span>
	{/if}
	{#if title}
		<div class="text-sm font-bold text-primary pr-2">{title}</div>
	{/if}
	{@render children?.()}
	{@render action?.()}
</div>
