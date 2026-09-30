<script lang="ts" module>
	import type { ComponentType } from 'svelte'
	import type { NativeTriggerKind } from './types'

	// Trigger kinds the pipeline graph can render as a source node. Union of
	// 'schedule' (inline cron) and the eight native-trigger keywords.
	export type TriggerNodeKind = 'schedule' | NativeTriggerKind

	// Per-kind presentation. Icons are kept loose — pick the lucide glyph
	// whose shape most-obviously signals the trigger type at a glance.
	import {
		Clock,
		Database,
		Mail,
		MessageSquare,
		Radio,
		Send,
		Webhook,
		Zap,
		CloudCog,
		Upload
	} from 'lucide-svelte'

	type Presentation = {
		icon: ComponentType
		label: string
		iconText: string
	}

	// One muted icon color for every source kind — colors are meaningful, not
	// decorative (brand guidelines), so the kind is carried by the icon and
	// label. Red stays reserved for the missing-trigger state.
	const MUTED = { iconText: 'text-secondary' }

	export const TRIGGER_NODE_STYLE: Record<TriggerNodeKind, Presentation> = {
		schedule: { icon: Clock, label: 'Schedule', ...MUTED },
		webhook: { icon: Webhook, label: 'Webhook', ...MUTED },
		email: { icon: Mail, label: 'Email', ...MUTED },
		kafka: { icon: Zap, label: 'Kafka', ...MUTED },
		mqtt: { icon: Radio, label: 'MQTT', ...MUTED },
		amqp: { icon: Radio, label: 'AMQP', ...MUTED },
		nats: { icon: MessageSquare, label: 'NATS', ...MUTED },
		postgres: { icon: Database, label: 'Postgres', ...MUTED },
		sqs: { icon: Send, label: 'SQS', ...MUTED },
		gcp: { icon: CloudCog, label: 'GCP Pub/Sub', ...MUTED },
		data_upload: { icon: Upload, label: 'Data upload', ...MUTED }
	}
</script>

<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import PipelineNodeCard from './PipelineNodeCard.svelte'
	import { describeCron } from '$lib/utils/describeCron'
	import { twMerge } from 'tailwind-merge'
	import { AlertTriangle, CheckCircle2, EllipsisVertical, Target, Trash2 } from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { stopPropagation, preventDefault } from 'svelte/legacy'
	import type { Item } from '$lib/utils'

	interface Props {
		// `ref` is the cron expression for schedules, the trigger-path for
		// attached native triggers, and a synthetic `missing:<script>` for
		// placeholders. `missing: true` swaps the styling to a red broken
		// state and surfaces "no trigger row" instead of a path; the
		// owning script is in `runnable_path` (used by the title and the
		// "+ Create trigger" drawer hook passed by the page).
		data: {
			kind: TriggerNodeKind
			ref: string
			unsaved?: boolean
			missing?: boolean
			// A pipeline-local schedule draft: `ref` is the path it deploys to.
			// Edit/delete go through the same callbacks, which the page routes
			// to the draft instead of a trigger row.
			draft?: boolean
			// The cron, for a schedule: the node names its cadence instead of "Schedule".
			schedule?: string
			runnable_path?: string
			// Page-supplied dispatcher that opens the matching native
			// trigger drawer with `script_path` pre-filled. When absent
			// (e.g. schedule, or a kind without an editor) the placeholder is
			// non-clickable. Webhook is handled separately via `onOpenWebhook`.
			onCreateMissingTrigger?: (kind: NativeTriggerKind, scriptPath: string) => void
			// Page-supplied dispatcher that opens the webhook drawer (URLs +
			// webhook-specific token creation) for `scriptPath`. Webhooks have
			// no trigger row, so they never use the create/edit/delete flows —
			// the node is always clickable to view its endpoint instead of
			// rendering a dead "missing" placeholder.
			onOpenWebhook?: (scriptPath: string) => void
			// Page-supplied dispatcher that opens the run form for a
			// `data_upload` source. Like webhook, data_upload has no trigger
			// row — it's a UI-first entry point, so the node is always
			// clickable and opens the script's run form (where the
			// auto-generated S3 picker lets the user upload + run) instead of
			// rendering a "missing" placeholder.
			onOpenDataUpload?: (scriptPath: string) => void
			// True once a file is staged for this data_upload entry — renders the
			// node green so the user knows the pipeline can run (see WIN-2129).
			ready?: boolean
			// Page-supplied dispatcher to open the matching native trigger
			// drawer in edit mode for an attached (non-missing) trigger.
			// `triggerPath` is the trigger row's path (e.g. the mqtt_trigger
			// row); `scriptPath` is the script the trigger targets — the
			// drawer locks its script-picker to this so the user can't
			// reassign the trigger off the pipeline. Absent for kinds
			// without an editor.
			onEditTrigger?: (kind: NativeTriggerKind, triggerPath: string, scriptPath: string) => void
			// Page-supplied dispatcher to delete an attached (non-missing)
			// trigger. Confirmation is the caller's responsibility — the
			// node just exposes the entry point on the kebab menu.
			onDeleteTrigger?: (kind: NativeTriggerKind, triggerPath: string) => void
			// Wired by the canvas only when this trigger's target script is a
			// valid bounded-run start (schedule / manual root) with downstream.
			// Entering the page's end-node pick mode rooted at that script — the
			// View-mode entry point for bounded runs (the script's own Run-button
			// caret is Edit-only).
			onStartBoundedRun?: () => void
		}
	}
	let { data }: Props = $props()

	let hover = $state(false)
	let menuOpen = $state(false)

	let style = $derived(TRIGGER_NODE_STYLE[data.kind])
	// Webhooks are never genuinely "missing" — every deployed runnable has an
	// implicit endpoint. Suppress the broken/red treatment for them so they
	// render as a normal clickable source node.
	let isWebhook = $derived(data.kind === 'webhook')
	// data_upload, like webhook, has no trigger row — it's a UI-first entry
	// point. Never render it as a red "missing" placeholder; it's always a
	// clickable source that opens the run form.
	let isDataUpload = $derived(data.kind === 'data_upload')
	let displayMissing = $derived(data.missing && !isWebhook && !isDataUpload)
	let Icon = $derived(displayMissing ? AlertTriangle : style.icon)
	let missingTitle = $derived(
		displayMissing
			? `Missing ${style.label} trigger: ${data.runnable_path ?? ''} declares \`// on ${data.kind}\` but no ${style.label} trigger targets it. Click to create one, or remove the annotation.`
			: undefined
	)
	// Webhook gets a drawer (URLs + webhook-specific token creation) rather
	// than a create/edit flow, so it's clickable whenever the page supplies
	// the handler.
	let canOpenWebhook = $derived(isWebhook && !!data.runnable_path && !!data.onOpenWebhook)
	// data_upload routes through its own handler — clicking opens the target
	// script's run form (with the auto-generated S3 picker).
	let canOpenDataUpload = $derived(isDataUpload && !!data.runnable_path && !!data.onOpenDataUpload)
	// A staged upload turns the node green (ready to run); before that it stays
	// on the neutral surface with the "upload a file" prompt.
	let dataUploadReady = $derived(isDataUpload && data.ready === true)
	let DataUploadIcon = $derived(dataUploadReady ? CheckCircle2 : style.icon)
	// Schedule + the other native kinds all have dedicated editors. Webhook and
	// data_upload are excluded — they route through their own open handlers.
	let canCreate = $derived(
		data.missing &&
			data.kind !== 'webhook' &&
			data.kind !== 'data_upload' &&
			!!data.runnable_path &&
			!!data.onCreateMissingTrigger
	)
	// Attached native trigger → clickable to open its drawer in edit mode.
	let canEdit = $derived(
		!data.missing &&
			data.kind !== 'webhook' &&
			data.kind !== 'data_upload' &&
			!!data.ref &&
			!!data.runnable_path &&
			!!data.onEditTrigger
	)

	// Same gating as `canEdit`: the trigger row only exists when there's a
	// non-missing ref + a backing editor (i.e. excludes webhook + data_upload).
	// Schedule has its own delete endpoint, same shape as the other natives.
	let canDelete = $derived(
		!data.missing &&
			data.kind !== 'webhook' &&
			data.kind !== 'data_upload' &&
			!!data.ref &&
			!!data.onDeleteTrigger
	)

	let menuItems: Item[] = $derived([
		...(data.onStartBoundedRun
			? [
					{
						displayName: 'Run + downstream…',
						icon: Target,
						action: () => data.onStartBoundedRun?.()
					}
				]
			: []),
		...(canDelete
			? [
					{
						displayName: data.draft ? 'Discard draft' : 'Delete…',
						icon: Trash2,
						type: 'delete' as const,
						action: () => {
							if (!data.ref || !data.onDeleteTrigger) return
							data.onDeleteTrigger?.(data.kind as NativeTriggerKind, data.ref)
						}
					}
				]
			: [])
	])

	// A schedule names its cadence ("Every day at 4:00", or the cron itself
	// when it has no plain reading); every other kind names its kind.
	let kindName = $derived(
		data.kind === 'schedule' && data.schedule
			? (describeCron(data.schedule) ?? `Schedule (${data.schedule})`)
			: style.label
	)

	// One card for every state: which label, title and click the node gets.
	let card = $derived.by(
		(): {
			kindLabel: string
			title: string
			tooltip?: string
			draft: boolean
			tone?: 'danger' | 'success'
			onclick?: () => void
		} => {
			const unsaved = !!data.unsaved
			if (canCreate)
				return {
					kindLabel: `${style.label} · missing`,
					title: 'Click to create',
					tooltip: missingTitle,
					draft: false,
					tone: 'danger',
					onclick: handleMissingClick
				}
			if (canEdit)
				return {
					kindLabel: `${kindName}${data.draft ? ' · draft' : unsaved ? ' · unsaved' : ''}`,
					title: data.ref,
					tooltip: data.draft
						? `Draft ${style.label}: ${data.ref} — created when you save the pipeline. Click to edit.`
						: `Edit ${style.label} trigger: ${data.ref}`,
					draft: unsaved,
					onclick: handleEditClick
				}
			if (canOpenWebhook)
				return {
					kindLabel: style.label,
					title: 'URLs & token',
					tooltip: `Webhook endpoint for ${data.runnable_path ?? ''} — click to view URLs and create a token`,
					draft: unsaved,
					onclick: handleWebhookClick
				}
			if (canOpenDataUpload)
				return {
					kindLabel: `${style.label}${dataUploadReady ? ' · ready' : ''}`,
					title: dataUploadReady ? 'File staged' : 'Upload & run',
					tooltip: dataUploadReady
						? `Data upload for ${data.runnable_path ?? ''} — a file is staged; the pipeline is ready to run. Click to change it.`
						: `Data upload for ${data.runnable_path ?? ''} — click to open the run form and upload a file`,
					draft: !dataUploadReady && unsaved,
					tone: dataUploadReady ? 'success' : undefined,
					onclick: handleDataUploadClick
				}
			return {
				kindLabel: `${displayMissing ? style.label : kindName}${displayMissing ? ' · missing' : unsaved ? ' · unsaved' : ''}`,
				title: displayMissing ? 'no trigger row' : data.ref,
				tooltip:
					missingTitle ??
					(unsaved ? `Unsaved ${style.label}: ${data.ref}` : `${style.label}: ${data.ref}`),
				draft: unsaved && !displayMissing,
				tone: displayMissing ? 'danger' : undefined
			}
		}
	)

	function handleMissingClick() {
		if (!canCreate || !data.runnable_path || !data.onCreateMissingTrigger) return
		data.onCreateMissingTrigger(data.kind as NativeTriggerKind, data.runnable_path)
	}

	function handleEditClick() {
		if (!canEdit || !data.ref || !data.runnable_path || !data.onEditTrigger) return
		data.onEditTrigger(data.kind as NativeTriggerKind, data.ref, data.runnable_path)
	}

	function handleWebhookClick() {
		if (!canOpenWebhook || !data.runnable_path || !data.onOpenWebhook) return
		data.onOpenWebhook(data.runnable_path)
	}

	function handleDataUploadClick() {
		if (!canOpenDataUpload || !data.runnable_path || !data.onOpenDataUpload) return
		data.onOpenDataUpload(data.runnable_path)
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="relative" onmouseenter={() => (hover = true)} onmouseleave={() => (hover = false)}>
	<PipelineNodeCard
		kindLabel={card.kindLabel}
		title={card.title}
		tooltip={card.tooltip}
		draft={card.draft}
		tone={card.tone}
		onclick={card.onclick}
	>
		{#snippet icon()}
			{#if canOpenDataUpload}
				<DataUploadIcon
					size={14}
					class={dataUploadReady ? 'text-green-600 dark:text-green-400' : style.iconText}
				/>
			{:else}
				<Icon
					size={14}
					class={displayMissing || canCreate ? 'text-red-600 dark:text-red-400' : style.iconText}
				/>
			{/if}
		{/snippet}
	</PipelineNodeCard>

	{#if menuItems.length > 0}
		<!-- Hover-revealed kebab menu (Delete only for now). Mirrors the
		     RunnableNode pattern: positioned just outside the top-right of
		     the node, rendered only on hover or while the menu is open so
		     the canvas stays clean at rest. `pointerdown` is stopped so
		     svelte-flow doesn't kick off node selection / drag when the
		     user reaches for the menu. -->
		<div class="absolute -top-2 -right-2 h-7 p-1 min-w-7" style="will-change: transform;">
			<DropdownV2
				items={menuItems}
				placement="bottom-end"
				bind:open={menuOpen}
				fixedHeight={false}
				usePointerDownOutside
			>
				{#snippet buttonReplacement()}
					<button
						class={twMerge(
							'center-center p-1 text-secondary shadow-sm bg-surface duration-0 hover:bg-surface-tertiary',
							hover || menuOpen ? 'block' : '!hidden',
							'shadow-md rounded-md'
						)}
						onpointerdown={stopPropagation(preventDefault(() => {}))}
						title="Actions"
					>
						<EllipsisVertical size={12} />
					</button>
				{/snippet}
			</DropdownV2>
		</div>
	{/if}
</div>

<Handle type="source" position={Position.Bottom} isConnectable={false} />
