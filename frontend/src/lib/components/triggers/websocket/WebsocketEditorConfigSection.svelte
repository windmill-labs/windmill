<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Required from '$lib/components/Required.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { sendUserToast } from '$lib/utils'
	import type { Schema } from '$lib/common'
	import { FlowService, ScriptService, type Flow, type Script } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'
	import TestingBadge from '$lib/components/triggers/testingBadge.svelte'

	interface Props {
		url: string | undefined
		url_runnable_args: Record<string, unknown> | undefined
		dirtyUrl?: boolean
		can_write?: boolean
		headless?: boolean
		isValid?: boolean
		showTestingBadge?: boolean
	}

	let {
		url = $bindable(),
		url_runnable_args = $bindable(),
		dirtyUrl = $bindable(false),
		can_write = false,
		headless = false,
		isValid = $bindable(false),
		showTestingBadge = false
	}: Props = $props()

	let areRunnableArgsValid: boolean = $state(true)

	let urlRunnableSchema: Schema | undefined = $state(undefined)
	async function loadUrlRunnableSchema(url: string | undefined) {
		if (url?.startsWith('$')) {
			const path = url.split(':')[1]
			if (path && path.length > 0) {
				try {
					let scriptOrFlow: Script | Flow = url.startsWith('$flow:')
						? await FlowService.getFlowByPath({
								workspace: $workspaceStore!,
								path: url.split(':')[1]
							})
						: await ScriptService.getScriptByPath({
								workspace: $workspaceStore!,
								path: url.split(':')[1]
							})
					urlRunnableSchema = scriptOrFlow.schema as Schema
				} catch (err) {
					sendUserToast(
						`Could not query runnable schema for ${url.startsWith('$flow:') ? 'flow' : 'script'} ${
							url.split(':')[1]
						}: ${err}`,
						true
					)
				}
			}
		}
	}
	$effect(() => {
		loadUrlRunnableSchema(url)
	})

	let urlError: string = $state('')
	let validateTimeout: number | undefined = undefined
	function validateUrl(url: string | undefined) {
		if (validateTimeout) {
			clearTimeout(validateTimeout)
		}
		validateTimeout = setTimeout(() => {
			if (url?.startsWith('$')) {
				if (/^(\$script|\$flow):[^\s]+$/.test(url) === false) {
					urlError = 'Invalid runnable path'
				} else {
					urlError = ''
				}
			} else if (!url || /^(ws:|wss:)\/\/[^\s]+$/.test(url) === false) {
				urlError = 'Websocket URL must start with ws:// or wss://'
			} else {
				urlError = ''
			}
			validateTimeout = undefined
		}, 500)
	}
	$effect(() => {
		validateUrl(url)
	})

	$effect(() => {
		isValid = urlError === '' && (!url?.startsWith('$') || areRunnableArgsValid)
	})
</script>

<div>
	<Section label="WebSocket" {headless}>
		{#snippet header()}
			{#if showTestingBadge}
				<TestingBadge />
			{/if}
		{/snippet}
		<div class="mb-2">
			<ToggleButtonGroup
				selected={url?.startsWith('$') ? 'runnable' : 'static'}
				on:selected={(ev) => {
					url = ev.detail === 'runnable' ? '$script:' : ''
					url_runnable_args = {}
				}}
				disabled={!can_write}
			>
				{#snippet children({ item, disabled })}
					<ToggleButton value="static" label="Static URL" {item} {disabled} />
					<ToggleButton value="runnable" label="Runnable result as URL" {item} {disabled} />
				{/snippet}
			</ToggleButtonGroup>
		</div>
		{#if url?.startsWith('$')}
			<div class="flex flex-col w-full gap-4">
				<div class="block grow w-full">
					<div class="text-secondary text-sm flex items-center gap-1 w-full justify-between">
						<div>
							Runnable
							<Required required={true} />
						</div>
					</div>
					<ScriptPicker
						allowFlow={true}
						itemKind={url.startsWith('$flow:') ? 'flow' : 'script'}
						initialPath={url.split(':')[1] ?? ''}
						on:select={(ev) => {
							dirtyUrl = true
							const { path, itemKind } = ev.detail
							url = `$${itemKind}:${path ?? ''}`
						}}
						disabled={!can_write}
					/>
					<div class="text-red-600 dark:text-red-400 text-2xs mt-1.5">
						{dirtyUrl ? urlError : ''}
					</div>
				</div>
			</div>

			{#if url.split(':')[1]?.length > 0}
				{#if urlRunnableSchema}
					<p class="font-semibold text-sm mt-4 mb-2">Arguments</p>
					{#await import('$lib/components/SchemaForm.svelte')}
						<Loader2 class="animate-spin mt-2" />
					{:then Module}
						{#key urlRunnableSchema}
							<Module.default
								schema={urlRunnableSchema}
								bind:args={url_runnable_args}
								bind:isValid={areRunnableArgsValid}
								shouldHideNoInputs
								className="text-xs"
								disabled={!can_write}
							/>
						{/key}
					{/await}
					{#if urlRunnableSchema.properties && Object.keys(urlRunnableSchema.properties).length === 0}
						<div class="text-xs text-secondary">This runnable takes no arguments</div>
					{/if}
				{:else}
					<Loader2 class="animate-spin mt-2" />
				{/if}
			{/if}
		{:else}
			<div class="flex flex-col w-full gap-4">
				<label class="block grow w-full">
					<div class="flex flex-col gap-1">
						<div class="text-secondary text-sm flex items-center gap-1 w-full justify-between">
							<div>
								URL
								<Required required={true} />
							</div>
						</div>
						<input
							type="text"
							autocomplete="off"
							bind:value={url}
							disabled={!can_write}
							oninput={() => {
								dirtyUrl = true
							}}
							placeholder="ws://example.com"
							class={urlError === ''
								? ''
								: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
						/>
						<div class="text-red-600 dark:text-red-400 text-2xs mt-1.5">
							{dirtyUrl ? urlError : ''}
						</div>
					</div>
				</label>
			</div>
		{/if}

		{#if isValid}
			<TestTriggerConnection kind="websocket" args={{ url, url_runnable_args }} />
		{/if}
	</Section>
</div>
