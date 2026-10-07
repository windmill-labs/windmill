<script lang="ts">
	import {
		computeSharableHash as computeSharableHash,
		defaultIfEmptyString,
		emptyString,
		truncateHash,
		sendUserToast
	} from '$lib/utils'

	import type { Schema } from '$lib/common'
	import { Badge, Button } from './common'
	import SchemaForm from './SchemaForm.svelte'
	import SharedBadge from './SharedBadge.svelte'

	import TimeAgo from './TimeAgo.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import { Calendar, Check, CornerDownLeft } from 'lucide-svelte'
	import RunFormAdvancedPopup from './RunFormAdvancedPopup.svelte'
	import { page } from '$app/state'
	import { replaceState } from '$app/navigation'
	import JsonInputs from '$lib/components/JsonInputs.svelte'
	import { argsToJsonPayload } from '$lib/schema'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import InputSelectedBadge from './schema/InputSelectedBadge.svelte'
	import { tick, untrack } from 'svelte'
	import { processSecretArgs } from './secretArgUtils'
	import { enforceDisabledDefaults, resetKeysToast } from './job_args'
	import PowerShellCommonParams from './PowerShellCommonParams.svelte'
	import { anyEditorUnparseable, flushAllPendingEditorChanges } from './pendingEditorFlush'
	import { useOperatingWorkspace } from './operatingWorkspace.svelte'

	// The ephemeral secret variable a password argument mints has to be created in the same
	// workspace the job runs in: a form embedded in a session runs the job in the session's
	// workspace, and a `$var:` minted in the navigation workspace resolves to nothing there.
	const operatingWorkspace = useOperatingWorkspace()
	let formEl: HTMLElement | undefined = $state()

	let reloadArgs = $state(0)
	let jsonEditor: JsonInputs | undefined = $state(undefined)
	let schemaHeight = $state(0)
	let showInputSelectedBadge = $state(false)
	let savedPreviousArgs: Record<string, any> | undefined = $state(undefined)
	let psCommonParams: Record<string, any> = $state({})
	// Reset on a view switch, where the editor that refused the run is gone, and on a form
	// validity change, where the field's own error and the disabled button take over.
	let blockedByUnparseable = $derived.by(() => {
		void jsonView
		void isValid
		return false
	})

	function extractPsCommonParams(allArgs: Record<string, any>): {
		scriptArgs: Record<string, any>
		commonParams: Record<string, any>
	} {
		const scriptArgs: Record<string, any> = {}
		const commonParams: Record<string, any> = {}
		for (const [k, v] of Object.entries(allArgs)) {
			if (k.startsWith('_wm_ps_')) {
				commonParams[k] = v
			} else {
				scriptArgs[k] = v
			}
		}
		return { scriptArgs, commonParams }
	}

	export async function setArgs(nargs: Record<string, any>) {
		// Only when this form is the one offering them: taking them aside is what lets the
		// section edit them, and a form without that section has nowhere to put them back
		// from. It does not keep them — `SchemaForm` holds `args` to the keys the schema
		// declares — so a form that hides the section neither offers these nor carries them.
		const { scriptArgs, commonParams } = showPsCommonParams
			? extractPsCommonParams(nargs)
			: { scriptArgs: nargs, commonParams: {} }
		args = scriptArgs
		psCommonParams = commonParams
		reloadArgs++
		// `reloadArgs` only keys the form; the JSON editor reads its payload once, at mount.
		syncJsonEditor()
	}

	export async function run(overrideScheduledForStr?: string | undefined | null) {
		// An editor whose text does not parse never wrote it to `args`, so running now would send
		// the last value that did parse. Flush first: a keystroke still inside the editor debounce
		// has not been parsed yet, and per-field editors parse it in an effect, hence the tick.
		flushAllPendingEditorChanges()
		await tick()
		blockedByUnparseable = anyEditorUnparseable(formEl)
		if (blockedByUnparseable) {
			return
		}
		// Both captured for the whole press, never read again after the await below: they stand
		// for whatever the press was started against — a chat tool call waiting on this form —
		// and that can settle and be replaced mid-press. A press that picked up the replacement
		// would run it without its reader ever confirming it, and would release its guard.
		const claim = claimRun
		const action = runAction
		if (claim && !claim.claim()) return
		let processedArgs: Record<string, any>
		const { args: withDefaults, resetKeys } = enforceDisabledDefaults(args ?? {}, runnable?.schema)
		if (resetKeys.length > 0) {
			sendUserToast(resetKeysToast(resetKeys))
		}
		try {
			processedArgs = await processSecretArgs(withDefaults, runnable?.schema, $operatingWorkspace)
		} catch (e) {
			claim?.release()
			sendUserToast('Failed to process sensitive args: ' + e, true)
			return
		}
		if (showPsCommonParams) {
			for (const [k, v] of Object.entries(psCommonParams)) {
				if (v !== undefined && v !== false && v !== '') {
					processedArgs[k] = v
				}
			}
		}
		action(
			overrideScheduledForStr === null ? undefined : (overrideScheduledForStr ?? scheduledForStr),
			processedArgs,
			invisible_to_owner,
			overrideTag
		)
	}

	interface Props {
		runnable:
			| {
					summary?: string
					schema?: Schema | any
					description?: string
					path?: string
					is_template?: boolean
					hash?: string
					kind?: string
					language?: string
					can_write?: boolean
					created_at?: string
					created_by?: string
					extra_perms?: Record<string, boolean>
			  }
			| undefined
		runAction: (
			scheduledForStr: string | undefined,
			args: Record<string, any>,
			invisible_to_owner: boolean | undefined,
			overrideTag: string | undefined
		) => void
		/** Asked before anything is written, for a `runAction` whose own guards must beat
		 * `processSecretArgs` to the workspace: false abandons the press. `release` is called
		 * if the press then ends before `runAction`, so a guard that counts presses can stop
		 * counting this one. */
		claimRun?: { claim: () => boolean; release: () => void }
		/** Whether the PowerShell common-parameter section is offered. Off for a form standing
		 * in for a chat card, which carries arguments and nothing else. */
		commonParams?: boolean
		/** Take no writes from the reader: no edits, and no dynamic-select helper. Both write
		 * before Run is ever pressed — a `password` field mints an ephemeral variable as it is
		 * typed, and the helper runs the `dynselect-` entrypoint when the field mounts — so
		 * nothing about gating Run can stop them. For a form standing in for a chat card while
		 * plan mode is on, which promised neither. */
		argsReadonly?: boolean
		buttonText?: string
		schedulable?: boolean
		detailed?: boolean
		autofocus?: boolean
		loading?: boolean
		noVariablePicker?: boolean
		viewKeybinding?: boolean
		scheduledForStr?: string | undefined
		invisible_to_owner?: boolean | undefined
		overrideTag?: string | undefined
		overrideTagNote?: string
		args?: Record<string, any>
		jsonView?: boolean
		isValid?: boolean
		/** Mirror the current args into the page URL's fragment, which is what makes a
		 * filled-in form shareable and what `Run again` reads back. Turn off wherever this
		 * form is embedded in a page that is not the runnable's own — an AI session preview
		 * tab — since there the fragment would land on an unrelated URL. */
		syncArgsToUrl?: boolean
		/** Controls beside the Run button: left of Advanced on a schedulable form, in
		 *  Advanced's place on one that cannot schedule. */
		actions?: import('svelte').Snippet
	}

	let {
		runnable,
		runAction,
		claimRun = undefined,
		commonParams = true,
		argsReadonly = false,
		buttonText = 'Run',
		schedulable = true,
		detailed = true,
		autofocus = false,
		loading = false,
		noVariablePicker = false,
		viewKeybinding = false,
		scheduledForStr = $bindable(),
		invisible_to_owner = $bindable(),
		overrideTag = $bindable(),
		overrideTagNote = undefined,
		args = $bindable(),
		jsonView = false,
		isValid = $bindable(true),
		syncArgsToUrl = true,
		actions = undefined
	}: Props = $props()

	// Kept out while the form stands in for a chat card: a call carries arguments and nothing
	// else, so these would be chosen here and then dropped when the run is confirmed anywhere
	// else — the same reason scheduling and the tag override are hidden.
	let showPsCommonParams = $derived(
		commonParams &&
			runnable?.language === 'powershell' &&
			runnable?.schema?.['x-windmill-ps-cmd-binding'] === true
	)

	$effect.pre(() => {
		if (args == undefined) {
			args = {}
		}
		// Extract _wm_ps_* keys from args on initial load (e.g. "Run again" via URL hash),
		// and only when this form offers the section — see `setArgs`.
		if (showPsCommonParams && args && Object.keys(args).some((k) => k.startsWith('_wm_ps_'))) {
			const { scriptArgs, commonParams } = extractPsCommonParams(args)
			args = scriptArgs
			psCommonParams = commonParams
		}
	})

	let debounced: number | undefined = undefined

	function onArgsChange(args: any) {
		if (!syncArgsToUrl) return
		try {
			debounced && clearTimeout(debounced)
			debounced = setTimeout(() => {
				const nurl = new URL(window.location.href)
				nurl.hash = computeSharableHash(args, overrideTag)

				try {
					replaceState(nurl.toString(), page.state)
				} catch (e) {
					console.error(e)
				}
			}, 200)
		} catch (e) {
			console.error('Impossible to set hash in args', e)
		}
	}

	/** Rewrite the open JSON editor from the current args. Only for args replaced from outside
	 * the editor: entering the JSON view already starts from whatever `args` holds. */
	export function syncJsonEditor() {
		jsonEditor?.setCode(argsToJsonPayload(runnable?.schema, args))
	}
	$effect(() => {
		overrideTag
		Object.keys(args ?? {}).forEach((key) => {
			args?.[key]
		})
		untrack(() => onArgsChange(args))
	})
</script>

<!-- Standalone triggerable registration for the run form -->
<div
	style="display: none"
	use:triggerableByAI={{
		id: `run-form-${runnable?.path ?? ''}`,
		description: `Form to fill the inputs to run ${runnable?.summary && runnable?.summary.length > 0 ? runnable?.summary : runnable?.path}.
	## Script description: ${runnable?.description ?? ''}.
	## Schema used: ${JSON.stringify(runnable?.schema)}.
	## Current args: ${JSON.stringify(args)}}`,
		callback: (value) => {
			savedPreviousArgs = args
			setArgs(JSON.parse(value ?? '{}'))
			showInputSelectedBadge = true
		},
		showAnimation: false
	}}
></div>

{#snippet acceptButton()}
	<Button
		startIcon={{
			icon: Check
		}}
		size="xs2"
		btnClasses="border border-gray-200 dark:border-gray-600 !bg-surface text-primary"
		on:click={() => {
			showInputSelectedBadge = false
			savedPreviousArgs = undefined
		}}
	>
		Accept
	</Button>
{/snippet}

{#if showInputSelectedBadge}
	<InputSelectedBadge
		inputSelected="ai"
		labelColor="text-violet-800 dark:text-primary"
		className="dark:!bg-violet-800 !bg-violet-200 !border-violet-200 dark:!border-violet-800"
		{acceptButton}
		onReject={() => {
			setArgs(savedPreviousArgs ?? {})
			savedPreviousArgs = undefined
			showInputSelectedBadge = false
		}}
	/>
{/if}
<div bind:this={formEl} class="max-w-3xl">
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
							<span class="text-sm text-primary">
								{#if runnable}
									Edited <TimeAgo agoOnlyIfRecent date={runnable.created_at || ''} /> by {runnable.created_by ||
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
			<h1
				use:triggerableByAI={{
					id: 'run-form-loading',
					description: 'Run form is loading, should scan the page until this is gone'
				}}>Loading...</h1
			>
		{/if}
	{/if}
	{#if runnable?.schema}
		{#if jsonView}
			<div
				class="py-2"
				style="height: {!schemaHeight || schemaHeight < 600 ? 600 : schemaHeight}px"
				data-schema-picker
			>
				<JsonInputs
					bind:this={jsonEditor}
					on:select={(e) => {
						// The other way into the arguments, and the one the disabled fields do not
						// cover: a password set here is a plain literal until the schema field mounts
						// onto it, and that field mints on mount whether or not it is disabled.
						if (argsReadonly) return
						blockedByUnparseable = false
						if (e.detail) {
							args = enforceDisabledDefaults(e.detail, runnable?.schema).args
						}
					}}
					initialCode={argsToJsonPayload(runnable.schema, args)}
					updateOnBlur={false}
					placeholder={`Write args as JSON.<br/><br/>Example:<br/><br/>{<br/>&nbsp;&nbsp;"foo": "12"<br/>}`}
				/>
			</div>
		{:else if !runnable.schema.properties || Object.keys(runnable.schema.properties).length === 0}
			<div class="text-sm italic">{`This ${runnable.kind ?? 'runnable'} takes no arguments`}</div>
		{:else}
			{#key reloadArgs}
				<div bind:clientHeight={schemaHeight}>
					<SchemaForm
						helperScript={argsReadonly
							? undefined
							: {
									source: 'deployed',
									path: runnable.path!,
									runnable_kind: runnable.hash ? 'script' : 'flow'
								}}
						disabled={argsReadonly}
						prettifyHeader
						{noVariablePicker}
						{autofocus}
						schema={runnable.schema}
						bind:isValid
						bind:args
					/>
				</div>
			{/key}
		{/if}
	{:else}
		<div class="text-xs text-primary">No arguments</div>
	{/if}
	{#if showPsCommonParams}
		<div class="mt-4">
			<PowerShellCommonParams bind:args={psCommonParams} />
		</div>
	{/if}
	{#if schedulable}
		<div class="flex gap-2 items-start flex-wrap justify-between mt-2 md:mt-6">
			<div class="flex-row-reverse flex-wrap flex w-full gap-4">
				<Button
					id="run-form-run-button"
					{loading}
					variant="accent"
					unifiedSize="md"
					btnClasses="!inline-flex"
					disabled={!isValid && !jsonView}
					on:click={() => run()}
					shortCut={{ Icon: CornerDownLeft, hide: !viewKeybinding }}
				>
					{scheduledForStr ? 'Schedule to run later' : buttonText}
				</Button>
				<div>
					<Popover placement="bottom" closeButton usePointerDownOutside>
						{#snippet trigger()}
							<Button nonCaptureEvent startIcon={{ icon: Calendar }} unifiedSize="md" color="light">
								Advanced
							</Button>
						{/snippet}
						{#snippet content()}
							<RunFormAdvancedPopup
								bind:scheduledForStr
								bind:invisible_to_owner
								bind:overrideTag
								{runnable}
							/>
						{/snippet}
					</Popover>
				</div>
				{@render actions?.()}
			</div>
			{@render unparseableError()}
			{#if overrideTag}
				<div class="flex-row-reverse flex w-full text-primary text-sm">
					tag override: {overrideTag}
				</div>
			{:else if overrideTagNote}
				<div class="flex-row-reverse flex w-full text-secondary text-xs">
					{overrideTagNote}
				</div>
			{/if}
			{#if invisible_to_owner}
				<div class="flex-row-reverse flex w-full text-primary text-sm">
					Job will be invisible to owner
				</div>
			{/if}
		</div>
	{:else if actions}
		<!-- The schedulable row's layout, with the caller's controls where Advanced sits. -->
		<div class="flex-row-reverse flex-wrap flex w-full gap-4 mt-2 md:mt-6">
			<Button
				{loading}
				variant="accent"
				unifiedSize="md"
				btnClasses="!inline-flex"
				disabled={!isValid && !jsonView}
				on:click={() => run(null)}
				shortCut={{ Icon: CornerDownLeft, hide: !viewKeybinding }}
			>
				{buttonText}
			</Button>
			<div>{@render actions()}</div>
		</div>
		{@render unparseableError()}
	{:else}
		<Button
			btnClasses="!px-6 !py-1 w-full"
			variant="accent"
			disabled={!isValid && !jsonView}
			on:click={() => run(null)}
			shortCut={{ Icon: CornerDownLeft, hide: !viewKeybinding }}
		>
			{buttonText}
		</Button>
		{@render unparseableError()}
	{/if}
</div>

{#snippet unparseableError()}
	{#if blockedByUnparseable}
		<div class="flex-row-reverse flex w-full text-red-600 dark:text-red-400 text-xs mt-1">
			Some input is not valid JSON. Fix it before running.
		</div>
	{/if}
{/snippet}
