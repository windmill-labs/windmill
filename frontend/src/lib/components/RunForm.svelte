<script lang="ts">
	import {
		defaultIfEmptyString,
		emptyString,
		getModifierKey,
		getToday,
		truncateHash
	} from '$lib/utils'

	import type { Schema } from '$lib/common'
	import { userStore } from '$lib/stores'
	import CliHelpBox from './CliHelpBox.svelte'
	import { Badge, Button, Kbd } from './common'
	import SchemaForm from './SchemaForm.svelte'
	import SharedBadge from './SharedBadge.svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'
	import CollapseLink from './CollapseLink.svelte'
	import { SCRIPT_VIEW_SHOW_RUN_FROM_CLI, SCRIPT_VIEW_SHOW_SCHEDULE_RUN_LATER } from '$lib/consts'
	import TimeAgo from './TimeAgo.svelte'
	import ClipboardPanel from './details/ClipboardPanel.svelte'

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
		invisible_to_owner?: boolean
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

	export let args: Record<string, any> = {}

	let reloadArgs = 0

	export async function setArgs(nargs: Record<string, any>) {
		args = nargs
		reloadArgs++
	}

	export function run() {
		runAction(scheduledForStr, args, invisible_to_owner)
	}

	export let isValid = true

	let scheduledForStr: string | undefined
	let invisible_to_owner: false

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
			on:click={() => runAction(undefined, args)}
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
					on:click={() => runAction(scheduledForStr, args, invisible_to_owner)}
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
					<CollapseLink small text="Advanced" class="justify-end">
						<div class="flex flex-col gap-4 mt-2 border p-2">
							<div class="flex flex-col gap-2">
								{#if SCRIPT_VIEW_SHOW_SCHEDULE_RUN_LATER}
									<div class="border rounded-md p-3 pt-4">
										<div class="px-2 font-semibold text-sm">Schedule to run later</div>

										<div class="flex flex-row items-end">
											<div class="w-max md:w-2/3 mt-2 mb-1">
												<label for="run-time" />
												<input
													class="inline-block"
													type="datetime-local"
													id="run-time"
													name="run-scheduled-time"
													bind:value={scheduledForStr}
													min={getToday().toISOString().slice(0, 16)}
												/>
											</div>
											<Button
												variant="border"
												color="blue"
												size="sm"
												btnClasses="mx-2 mb-1"
												on:click={() => {
													scheduledForStr = undefined
												}}
											>
												Clear
											</Button>
										</div>
									</div>
								{/if}
							</div>
							{#if runnable?.path?.startsWith(`u/${$userStore?.username}`) != true && (runnable?.path?.split('/')?.length ?? 0) > 2}
								<div class="flex items-center gap-1">
									<Toggle
										options={{
											right: `make run invisible to others`
										}}
										bind:checked={invisible_to_owner}
									/>
									<Tooltip
										>By default, runs are visible to the owner(s) of the script or flow being
										triggered</Tooltip
									>
								</div>
							{/if}
						</div>
					</CollapseLink>
				</div>
			</div>
		</div>
	{:else if !topButton}
		<Button
			btnClasses="!px-6 !py-1 w-full"
			disabled={!isValid}
			on:click={() => runAction(undefined, args, invisible_to_owner)}
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
