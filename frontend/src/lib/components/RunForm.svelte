<script lang="ts">
	import { defaultIfEmptyString, emptyString, getModifierKey, truncateHash } from '$lib/utils'

	import type { Schema } from '$lib/common'
	import CliHelpBox from './CliHelpBox.svelte'
	import { Badge, Button, Kbd } from './common'
	import SchemaForm from './SchemaForm.svelte'
	import SharedBadge from './SharedBadge.svelte'

	import CollapseLink from './CollapseLink.svelte'
	import { SCRIPT_VIEW_SHOW_RUN_FROM_CLI } from '$lib/consts'
	import TimeAgo from './TimeAgo.svelte'
	import ClipboardPanel from './details/ClipboardPanel.svelte'
	import Popup from './common/popup/Popup.svelte'
	import { autoPlacement } from '@floating-ui/core'
	import { Calendar } from 'lucide-svelte'
	import RunFormAdvancedPopup from './RunFormAdvancedPopup.svelte'

	export let runnable:
		| {
				summary?: string
				schema?: Schema | any
				description?: string
				path?: string
				is_template?: boolean
				hash?: string
				kind?: string
				can_write?: boolean
				created_at?: string
				created_by?: string
				extra_perms?: Record<string, boolean>
		  }
		| undefined
	export let runAction: (
		scheduledForStr: string | undefined,
		args: Record<string, any>,
		invisible_to_owner: boolean | undefined,
		overrideTag: string | undefined
	) => void
	export let buttonText = 'Run'
	export let schedulable = true
	export let detailed = true
	export let autofocus = false
	export let topButton = false
	export let loading = false
	export let noVariablePicker = false
	export let viewCliRun = false
	export let isFlow: boolean
	export let viewKeybinding = false

	export let scheduledForStr: string | undefined
	export let invisible_to_owner: false | undefined
	export let overrideTag: string | undefined

	export let args: Record<string, any> = {}

	let reloadArgs = 0
	let blockPopupOpen = false

	export async function setArgs(nargs: Record<string, any>) {
		args = nargs
		reloadArgs++
	}

	export function run() {
		runAction(scheduledForStr, args, invisible_to_owner, overrideTag)
	}

	export let isValid = true

	$: cliCommand = `wmill ${isFlow ? 'flow' : 'script'} run ${runnable?.path} -d '${JSON.stringify(
		args
	)}'`
</script>

<div class="max-w-3xl">
	{#if detailed}
		{#if runnable}
			<div class="flex flex-row flex-wrap justify-between gap-4">
				<div>
					<div class="flex flex-col mb-2">
						<h1 class="break-words py-2 mr-2">
							{defaultIfEmptyString(runnable.summary, runnable.path ?? '')}
						</h1>
						{#if !emptyString(runnable.summary)}
							<h2 class="font-bold pb-4">{runnable.path}</h2>
						{/if}

						<div class="flex items-center gap-2">
							<span class="text-sm text-tertiary">
								{#if runnable}
									Edited <TimeAgo date={runnable.created_at || ''} /> by {runnable.created_by ||
										'unknown'}
								{/if}
							</span>
							<Badge color="dark-gray">
								{truncateHash(runnable?.hash ?? '')}
							</Badge>
							{#if runnable?.is_template}
								<Badge color="blue">Template</Badge>
							{/if}
							{#if runnable && runnable.kind !== 'runnable'}
								<Badge color="blue">
									{runnable?.kind}
								</Badge>
							{/if}
							<SharedBadge
								canWrite={runnable.can_write ?? true}
								extraPerms={runnable?.extra_perms ?? {}}
							/>
						</div>
					</div>
				</div>
			</div>
		{:else}
			<h1>Loading...</h1>
		{/if}
	{/if}
	{#if topButton}
		<Button
			btnClasses="!px-6 !py-1 w-full"
			disabled={!isValid}
			on:click={() => runAction(undefined, args, invisible_to_owner, overrideTag)}
		>
			{buttonText}
		</Button>
	{/if}
	{#if runnable?.schema}
		<div class="my-2" />
		{#if !runnable.schema.properties || Object.keys(runnable.schema.properties).length === 0}
			<div class="text-sm py-4 italic">No arguments</div>
		{:else}
			{#key reloadArgs}
				<SchemaForm
					prettifyHeader
					{noVariablePicker}
					{autofocus}
					schema={runnable.schema}
					bind:isValid
					bind:args
				/>
			{/key}
		{/if}
	{:else}
		<div class="text-xs text-tertiary">No arguments</div>
	{/if}
	{#if schedulable}
		<div class="mt-10" />
		<div class="flex gap-2 items-start flex-wrap justify-between mt-2 md:mt-6 mb-6">
			<div class="flex-row-reverse flex-wrap flex w-full gap-4">
				<Button
					{loading}
					color="dark"
					btnClasses="!px-6 !py-1 !h-8 inline-flex gap-2"
					disabled={!isValid}
					on:click={() => runAction(scheduledForStr, args, invisible_to_owner, overrideTag)}
				>
					{#if viewKeybinding}
						<div class="inline-flex gap-0 items-center">
							<Kbd small isModifier>{getModifierKey()}</Kbd><Kbd small>Enter</Kbd>
						</div>{/if}
					<div>
						{scheduledForStr ? 'Schedule to run later' : buttonText}
					</div>
				</Button>
				<div>
					<Popup
						floatingConfig={{
							middleware: [
								autoPlacement({
									allowedPlacements: [
										'bottom-start',
										'bottom-end',
										'top-start',
										'top-end',
										'top',
										'bottom'
									]
								})
							]
						}}
						let:close
						blockOpen={blockPopupOpen}
					>
						<svelte:fragment slot="button">
							<Button nonCaptureEvent startIcon={{ icon: Calendar }} size="xs" color="light"
								>Advanced</Button
							>
						</svelte:fragment>
						<RunFormAdvancedPopup
							bind:scheduledForStr
							bind:invisible_to_owner
							bind:overrideTag
							bind:runnable
							on:close={() => close(null)}
						/>
					</Popup>
				</div>
			</div>
			{#if overrideTag}
				<div class="flex-row-reverse flex w-full text-primary text-sm">
					tag override: {overrideTag}
				</div>
			{/if}
			{#if invisible_to_owner}
				<div class="flex-row-reverse flex w-full text-primary text-sm">
					Job will be invisible to owner
				</div>
			{/if}
		</div>
	{:else if !topButton}
		<Button
			btnClasses="!px-6 !py-1 w-full"
			disabled={!isValid}
			on:click={() => runAction(undefined, args, invisible_to_owner, overrideTag)}
		>
			{#if viewKeybinding}
				<div>
					<Kbd small isModifier>{getModifierKey()}</Kbd>+<Kbd small>Enter</Kbd>
				</div>{/if}
			{buttonText}
		</Button>
	{/if}

	{#if viewCliRun}
		<div>
			<div class="mt-4" />
			{#if SCRIPT_VIEW_SHOW_RUN_FROM_CLI}
				<CollapseLink small text="Run it from CLI">
					<div class="mt-2" />
					<ClipboardPanel content={cliCommand} />
					<CliHelpBox />
				</CollapseLink>
			{/if}
			<div class="mb-20" />
		</div>
	{/if}
</div>
