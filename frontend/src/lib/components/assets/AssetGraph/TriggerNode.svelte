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
		// Tailwind class fragments for bg + border + text accent.
		bg: string
		border: string
		borderUnsaved: string
		iconText: string
	}

	// One muted treatment for every source kind — colors are meaningful, not
	// decorative (brand guidelines), so the kind is carried by the icon and
	// label while bg/border stay on the surface/gray scale like the flow
	// editor's nodes. Red stays reserved for the missing-trigger state.
	const MUTED = {
		bg: 'bg-surface-secondary',
		border: 'outline-gray-400 dark:outline-gray-600',
		borderUnsaved: 'outline-dashed outline-gray-400 dark:outline-gray-500',
		iconText: 'text-secondary'
	}

	export const TRIGGER_NODE_STYLE: Record<TriggerNodeKind, Presentation> = {
		schedule: { icon: Clock, label: 'schedule', ...MUTED },
		webhook: { icon: Webhook, label: 'webhook', ...MUTED },
		email: { icon: Mail, label: 'email', ...MUTED },
		kafka: { icon: Zap, label: 'kafka', ...MUTED },
		mqtt: { icon: Radio, label: 'mqtt', ...MUTED },
		nats: { icon: MessageSquare, label: 'nats', ...MUTED },
		postgres: { icon: Database, label: 'postgres', ...MUTED },
		sqs: { icon: Send, label: 'sqs', ...MUTED },
		gcp: { icon: CloudCog, label: 'gcp', ...MUTED },
		data_upload: { icon: Upload, label: 'data upload', ...MUTED }
	}
</script>

<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import { NODE } from '$lib/components/graph/util'
	import { twMerge } from 'tailwind-merge'
	import { AlertTriangle, EllipsisVertical, Target, Trash2 } from 'lucide-svelte'
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
			runnable_path?: string
			// True iff the target script is still a draft (no DB row yet).
			// Drives the "(after draft save)" hint on the missing
			// placeholder, since the page-level handler refuses to open the
			// create drawer until the script is deployed.
			runnable_unsaved?: boolean
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
			? `Missing ${style.label} trigger: ${data.runnable_path ?? ''} declares \`// on ${style.label}\` but no ${style.label} trigger targets it. Click to create one, or remove the annotation.`
			: undefined
	)
	// Webhook gets a drawer (URLs + webhook-specific token creation) rather
	// than a create/edit flow, so it's clickable whenever the page supplies
	// the handler.
	let canOpenWebhook = $derived(isWebhook && !!data.runnable_path && !!data.onOpenWebhook)
	// data_upload routes through its own handler — clicking opens the target
	// script's run form (with the auto-generated S3 picker).
	let canOpenDataUpload = $derived(isDataUpload && !!data.runnable_path && !!data.onOpenDataUpload)
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
						displayName: 'Delete…',
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
	{#if canCreate}
		<!-- Same affordance as the asset's downstream + button: render the
		     whole node as a button so the cursor + hover + click is obvious
		     without depending on svelte-flow's wrapper-level handlers. -->
		<button
			type="button"
			onclick={handleMissingClick}
			class={twMerge(
				'flex items-center rounded-md drop-shadow-sm overflow-hidden outline outline-1 w-full text-left',
				'bg-red-50 dark:bg-red-900/30 outline-dashed outline-red-400 dark:outline-red-500',
				'hover:bg-red-100 dark:hover:bg-red-900/40 transition-colors'
			)}
			style="width: {NODE.width}px; min-height: {NODE.height}px;"
			title={missingTitle}
		>
			<Icon size={14} class="shrink-0 ml-2 mr-2 text-red-600 dark:text-red-400" />
			<div class="flex flex-col min-w-0 flex-1 pr-2 py-0.5 leading-tight">
				<span class="text-3xs uppercase tracking-wide truncate text-red-700 dark:text-red-400">
					{style.label} · missing
				</span>
				<span class="text-2xs font-mono truncate text-red-700 dark:text-red-400">
					{data.runnable_unsaved ? 'Click to create (after draft save)' : 'Click to create'}
				</span>
			</div>
		</button>
	{:else if canEdit}
		<!-- Mirrors the missing-trigger pattern: render the whole node as a
		     button so clicks open the editor drawer reliably (don't rely on
		     svelte-flow's onnodeclick wiring). -->
		<button
			type="button"
			onclick={handleEditClick}
			class={twMerge(
				'flex items-center rounded-md drop-shadow-sm overflow-hidden outline outline-1 w-full text-left',
				style.bg,
				data.unsaved ? `opacity-80 ${style.borderUnsaved}` : style.border,
				'hover:brightness-95 dark:hover:brightness-110 transition-[filter]'
			)}
			style="width: {NODE.width}px; min-height: {NODE.height}px;"
			title={`Edit ${style.label} trigger: ${data.ref}`}
		>
			<Icon size={14} class={`shrink-0 ml-2 mr-2 ${style.iconText}`} />
			<div class="flex flex-col min-w-0 flex-1 pr-2 py-0.5 leading-tight">
				<span class="text-3xs uppercase tracking-wide truncate text-tertiary">
					{style.label}{data.unsaved ? ' · unsaved' : ''}
				</span>
				<span class="text-2xs font-mono truncate text-emphasis">
					{data.ref}
				</span>
			</div>
		</button>
	{:else if canOpenWebhook}
		<!-- Webhook endpoint: implicit (no trigger row), so the node opens a
		     drawer with the URLs + webhook-specific token creation instead of
		     an editor. Styled like an attached trigger, never the red
		     "missing" state. -->
		<button
			type="button"
			onclick={handleWebhookClick}
			class={twMerge(
				'flex items-center rounded-md drop-shadow-sm overflow-hidden outline outline-1 w-full text-left',
				style.bg,
				data.unsaved ? `opacity-80 ${style.borderUnsaved}` : style.border,
				'hover:brightness-95 dark:hover:brightness-110 transition-[filter]'
			)}
			style="width: {NODE.width}px; min-height: {NODE.height}px;"
			title={`Webhook endpoint for ${data.runnable_path ?? ''} — click to view URLs and create a token`}
		>
			<Icon size={14} class={`shrink-0 ml-2 mr-2 ${style.iconText}`} />
			<div class="flex flex-col min-w-0 flex-1 pr-2 py-0.5 leading-tight">
				<span class="text-3xs uppercase tracking-wide truncate text-tertiary">
					{style.label}
				</span>
				<span class="text-2xs font-mono truncate text-emphasis"> URLs & token </span>
			</div>
		</button>
	{:else if canOpenDataUpload}
		<!-- Data upload: UI-first entry point (no trigger row). Clicking the
		     node opens the target script's run form, where the auto-generated
		     S3 picker lets the user upload a file and run the pipeline. Styled
		     like an attached trigger, never the red "missing" state. -->
		<button
			type="button"
			onclick={handleDataUploadClick}
			class={twMerge(
				'flex items-center rounded-md drop-shadow-sm overflow-hidden outline outline-1 w-full text-left',
				style.bg,
				data.unsaved ? `opacity-80 ${style.borderUnsaved}` : style.border,
				'hover:brightness-95 dark:hover:brightness-110 transition-[filter]'
			)}
			style="width: {NODE.width}px; min-height: {NODE.height}px;"
			title={`Data upload for ${data.runnable_path ?? ''} — click to open the run form and upload a file`}
		>
			<Icon size={14} class={`shrink-0 ml-2 mr-2 ${style.iconText}`} />
			<div class="flex flex-col min-w-0 flex-1 pr-2 py-0.5 leading-tight">
				<span class="text-3xs uppercase tracking-wide truncate text-tertiary">
					{style.label}
				</span>
				<span class="text-2xs font-mono truncate text-emphasis"> Upload & run </span>
			</div>
		</button>
	{:else}
		<div
			class={twMerge(
				'flex items-center rounded-md drop-shadow-sm overflow-hidden outline outline-1',
				displayMissing
					? 'bg-red-50 dark:bg-red-900/30 outline-dashed outline-red-400 dark:outline-red-500'
					: style.bg,
				displayMissing ? '' : data.unsaved ? `opacity-80 ${style.borderUnsaved}` : style.border
			)}
			style="width: {NODE.width}px; min-height: {NODE.height}px;"
			title={missingTitle ??
				(data.unsaved ? `Unsaved ${style.label}: ${data.ref}` : `${style.label}: ${data.ref}`)}
		>
			<Icon
				size={14}
				class={`shrink-0 ml-2 mr-2 ${displayMissing ? 'text-red-600 dark:text-red-400' : style.iconText}`}
			/>
			<div class="flex flex-col min-w-0 flex-1 pr-2 py-0.5 leading-tight">
				<span
					class={twMerge(
						'text-3xs uppercase tracking-wide truncate',
						displayMissing ? 'text-red-700 dark:text-red-400' : 'text-tertiary'
					)}
				>
					{style.label}{displayMissing ? ' · missing' : data.unsaved ? ' · unsaved' : ''}
				</span>
				<span
					class={twMerge(
						'text-2xs font-mono truncate',
						displayMissing ? 'text-red-700 dark:text-red-400' : 'text-emphasis'
					)}
				>
					{displayMissing ? 'no trigger row' : data.ref}
				</span>
			</div>
		</div>
	{/if}

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
