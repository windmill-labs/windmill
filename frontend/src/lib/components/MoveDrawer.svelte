<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Alert, Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Path from './Path.svelte'
	import { isOwner } from '$lib/utils'
	import { updateItemPathAndSummary, checkFlowOnBehalfOf } from './moveRenameManager'
	import Label from './Label.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import { FlowService, ScriptService, type TriggersCount } from '$lib/gen'

	const dispatch = createEventDispatcher()

	type Kind = 'script' | 'resource' | 'schedule' | 'variable' | 'flow' | 'app'

	let kind = $state<Kind>('flow')
	let initialPath = $state('')
	let initialSummary = $state('')
	let path = $state<string | undefined>(undefined)
	let summary = $state<string | undefined>(undefined)
	let dirtyPath = $state(false)

	let drawer = $state<Drawer>() as Drawer

	let own = $state(false)
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

	export async function openDrawer(
		initialPath_l: string,
		summary_l: string | undefined,
		kind_l: Kind
	) {
		kind = kind_l
		path = undefined
		dirtyPath = false
		onBehalfOfEmail = undefined
		attachedTriggers = undefined
		initialPath = initialPath_l
		initialSummary = summary_l ?? ''
		summary = summary_l
		loadOwner()
		drawer.openDrawer()
		if (kind === 'flow') {
			onBehalfOfEmail = await checkFlowOnBehalfOf($workspaceStore!, initialPath_l)
		}
		if (kind === 'script' || kind === 'flow') {
			void loadAttachedTriggers()
		}
	}

	async function loadAttachedTriggers() {
		try {
			const workspace = $workspaceStore!
			attachedTriggers =
				kind === 'flow'
					? await FlowService.getTriggersCountOfFlow({ workspace, path: initialPath })
					: await ScriptService.getTriggersCountOfScript({ workspace, path: initialPath })
		} catch {
			// Non-fatal: the rename still works without the summary panel.
			attachedTriggers = undefined
		}
	}

	function loadOwner() {
		own = isOwner(initialPath, $userStore!, $workspaceStore!)
	}

	async function updatePath() {
		if (kind === 'flow' || kind === 'script' || kind === 'app') {
			await updateItemPathAndSummary({
				workspace: $workspaceStore!,
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
		{#if !own}
			<Alert type="warning" title="Not owner" class="mb-4">
				Since you do not own this item, you cannot move this item (you can however fork it)
			</Alert>
		{/if}
		{#if own && onBehalfOfEmail}
			<Alert type="info" title="Run on behalf of" class="mb-4">
				This flow will be redeployed on behalf of you ({$userStore?.email}) instead of {onBehalfOfEmail}
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
							><span class="font-mono font-semibold">{count}</span> {label}{count === 1
								? ''
								: 's'}</span
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
