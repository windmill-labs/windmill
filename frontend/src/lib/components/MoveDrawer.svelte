<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { useOperatingWorkspace, useOperatingUser } from './operatingWorkspace.svelte'
	import { Alert, Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Path from './Path.svelte'
	import { isOwner } from '$lib/utils'
	import { updateItemPathAndSummary, checkFlowOnBehalfOf } from './moveRenameManager'
	import Label from './Label.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import { DraftService, FlowService, ScriptService, type TriggersCount } from '$lib/gen'

	const dispatch = createEventDispatcher()

	// A move is a write, so it has to land in the workspace the host acts on: inside an AI
	// session's editor or deployed view that is the session's (possibly forked) workspace,
	// while the navigation store still names the parent.
	const operatingWorkspace = useOperatingWorkspace()
	const operatingUser = useOperatingUser()

	type Kind = 'script' | 'resource' | 'schedule' | 'variable' | 'flow' | 'app'

	/** The address the move endpoint takes: where a draft-only item's row lives, which
	 * `initialPath` (what the user sees and edits) need not equal. Empty for a deployed
	 * item, which is addressed by `initialPath` instead. */
	let storagePath = $state('')
	let rawApp = $state(false)
	let draftOnly = $derived(storagePath !== '')

	let kind = $state<Kind>('flow')
	let initialPath = $state('')
	let initialSummary = $state('')
	let path = $state<string | undefined>(undefined)
	let summary = $state<string | undefined>(undefined)
	let dirtyPath = $state(false)

	let drawer = $state<Drawer>() as Drawer

	// Derived rather than read when the drawer opens: the acting user in another workspace is
	// looked up asynchronously, so a snapshot taken at open time says "not owner" for a fork.
	const ownerKnown = $derived(operatingUser.resolved($operatingWorkspace))
	const own = $derived(
		!!operatingUser.current &&
			isOwner(draftOnly ? storagePath : initialPath, operatingUser.current, $operatingWorkspace!)
	)
	let onBehalfOfEmail = $state<string | undefined>(undefined)
	// Counts of triggers/schedules/etc. that reference this script or flow.
	// The backend cascades `script_path` on rename across all trigger tables
	// (see `windmill_common::triggers::update_triggers_script_path` invoked
	// from script/flow create), so the user just needs to know what will be
	// moved along — not opt in per-row.
	let attachedTriggers = $state<TriggersCount | undefined>(undefined)
	let hasChanges = $derived((summary ?? '') !== initialSummary || dirtyPath)

	// Flatten the count buckets into a uniform list for rendering. Order
	// reflects user-facing prominence: schedules first (most common), then
	// the seven native trigger kinds, then HTTP / webhook / websocket /
	// email-default / cloud-service installations. Buckets with count 0
	// are dropped so the panel only mentions triggers that actually exist.
	let attachedSummary = $derived.by<Array<{ label: string; count: number }>>(() => {
		const c = attachedTriggers
		if (!c) return []
		const out: Array<{ label: string; count: number }> = []
		const push = (label: string, n: number | undefined) => {
			if (typeof n === 'number' && n > 0) out.push({ label, count: n })
		}
		push('schedule', c.schedule_count)
		push('kafka', c.kafka_count)
		push('mqtt', c.mqtt_count)
		push('amqp', c.amqp_count)
		push('nats', c.nats_count)
		push('postgres', c.postgres_count)
		push('sqs', c.sqs_count)
		push('gcp', c.gcp_count)
		push('email', c.email_count)
		push('http route', c.http_routes_count)
		push('websocket', c.websocket_count)
		push('webhook token', c.webhook_count)
		push('default-email token', c.default_email_count)
		push('nextcloud', c.nextcloud_count)
		push('google', c.google_count)
		push('github', c.github_count)
		push('azure', c.azure_count)
		return out
	})
	let attachedTotal = $derived(attachedSummary.reduce((s, { count }) => s + count, 0))

	/** `draft` marks an item that exists only as the caller's draft: pass the
	 * generated path its draft row sits at, and `initialPath_l` is then the name
	 * the user sees. Nothing is deployed, so there are no triggers to cascade
	 * and no on-behalf-of identity to warn about. */
	export async function openDrawer(
		initialPath_l: string,
		summary_l: string | undefined,
		kind_l: Kind,
		draft?: { storagePath: string; rawApp?: boolean }
	) {
		kind = kind_l
		path = undefined
		dirtyPath = false
		onBehalfOfEmail = undefined
		attachedTriggers = undefined
		storagePath = draft?.storagePath ?? ''
		rawApp = draft?.rawApp ?? false
		initialPath = initialPath_l
		initialSummary = summary_l ?? ''
		summary = summary_l
		drawer.openDrawer()
		if (draftOnly) {
			return
		}
		if (kind === 'flow') {
			onBehalfOfEmail = await checkFlowOnBehalfOf($operatingWorkspace!, initialPath_l)
		}
		if (kind === 'script' || kind === 'flow') {
			void loadAttachedTriggers()
		}
	}

	async function loadAttachedTriggers() {
		try {
			const workspace = $operatingWorkspace!
			attachedTriggers =
				kind === 'flow'
					? await FlowService.getTriggersCountOfFlow({ workspace, path: initialPath })
					: await ScriptService.getTriggersCountOfScript({ workspace, path: initialPath })
		} catch {
			// Non-fatal: the rename still works without the summary panel.
			attachedTriggers = undefined
		}
	}

	async function updatePath() {
		if (draftOnly && (kind === 'flow' || kind === 'script' || kind === 'app')) {
			await DraftService.moveDraft({
				workspace: $operatingWorkspace!,
				kind: kind === 'app' && rawApp ? 'raw_app' : kind,
				path: storagePath,
				requestBody: { new_path: path ?? '', summary: summary ?? '' }
			})
		} else if (kind === 'flow' || kind === 'script' || kind === 'app') {
			await updateItemPathAndSummary({
				workspace: $operatingWorkspace!,
				kind,
				initialPath,
				newPath: path ?? '',
				newSummary: summary ?? ''
			})
		}
		dispatch('update', path)
		drawer.closeDrawer()
	}
</script>

<Drawer bind:this={drawer}>
	<DrawerContent title="Move/Rename {initialPath}" on:close={drawer.closeDrawer}>
		<!-- Only once the acting user is known: while the lookup is in flight `own` is false,
		     and stating "you do not own this" then would be a guess. -->
		{#if ownerKnown && !own}
			<Alert type="warning" title="Not owner" class="mb-4">
				Since you do not own this item, you cannot move this item (you can however fork it)
			</Alert>
		{/if}
		{#if own && onBehalfOfEmail}
			<Alert type="info" title="Run on behalf of" class="mb-4">
				This flow will be redeployed on behalf of you ({operatingUser.current?.email}) instead of {onBehalfOfEmail}
			</Alert>
		{/if}
		{#if (kind === 'script' || kind === 'flow') && attachedTotal > 0}
			<Alert
				type="info"
				title={`Will also update ${attachedTotal} attached trigger${attachedTotal === 1 ? '' : 's'}`}
				class="mb-4"
			>
				<div class="flex flex-wrap gap-x-3 gap-y-1 mt-1">
					{#each attachedSummary as { label, count } (label)}
						<span class="text-xs"
							><span class="font-mono font-semibold">{count}</span>
							{label}{count === 1 ? '' : 's'}</span
						>
					{/each}
				</div>
			</Alert>
		{/if}
		<Label label="Summary" class="mb-6">
			<TextInput
				inputProps={{
					type: 'text',
					placeholder: 'Short summary to be displayed when listed',
					disabled: !own
				}}
				bind:value={summary}
			/>
		</Label>

		<Label label="Path">
			<Path disabled={!own} {kind} {initialPath} bind:path bind:dirty={dirtyPath} />
		</Label>
		{#snippet actions()}
			<Button variant="accent" disabled={!own || !hasChanges} on:click={updatePath}
				>Move/Rename</Button
			>
		{/snippet}
	</DrawerContent>
</Drawer>
