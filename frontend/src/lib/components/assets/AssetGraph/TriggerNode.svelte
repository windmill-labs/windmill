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
		CloudCog
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

	export const TRIGGER_NODE_STYLE: Record<TriggerNodeKind, Presentation> = {
		schedule: {
			icon: Clock,
			label: 'schedule',
			bg: 'bg-amber-50 dark:bg-amber-900/30',
			border: 'outline-amber-300 dark:outline-amber-600/60',
			borderUnsaved: 'outline-dashed outline-amber-400',
			iconText: 'text-amber-700 dark:text-amber-400'
		},
		webhook: {
			icon: Webhook,
			label: 'webhook',
			bg: 'bg-sky-50 dark:bg-sky-900/30',
			border: 'outline-sky-300 dark:outline-sky-600/60',
			borderUnsaved: 'outline-dashed outline-sky-400',
			iconText: 'text-sky-700 dark:text-sky-400'
		},
		email: {
			icon: Mail,
			label: 'email',
			bg: 'bg-violet-50 dark:bg-violet-900/30',
			border: 'outline-violet-300 dark:outline-violet-600/60',
			borderUnsaved: 'outline-dashed outline-violet-400',
			iconText: 'text-violet-700 dark:text-violet-400'
		},
		kafka: {
			icon: Zap,
			label: 'kafka',
			bg: 'bg-rose-50 dark:bg-rose-900/30',
			border: 'outline-rose-300 dark:outline-rose-600/60',
			borderUnsaved: 'outline-dashed outline-rose-400',
			iconText: 'text-rose-700 dark:text-rose-400'
		},
		mqtt: {
			icon: Radio,
			label: 'mqtt',
			bg: 'bg-teal-50 dark:bg-teal-900/30',
			border: 'outline-teal-300 dark:outline-teal-600/60',
			borderUnsaved: 'outline-dashed outline-teal-400',
			iconText: 'text-teal-700 dark:text-teal-400'
		},
		nats: {
			icon: MessageSquare,
			label: 'nats',
			bg: 'bg-cyan-50 dark:bg-cyan-900/30',
			border: 'outline-cyan-300 dark:outline-cyan-600/60',
			borderUnsaved: 'outline-dashed outline-cyan-400',
			iconText: 'text-cyan-700 dark:text-cyan-400'
		},
		postgres: {
			icon: Database,
			label: 'postgres',
			bg: 'bg-indigo-50 dark:bg-indigo-900/30',
			border: 'outline-indigo-300 dark:outline-indigo-600/60',
			borderUnsaved: 'outline-dashed outline-indigo-400',
			iconText: 'text-indigo-700 dark:text-indigo-400'
		},
		sqs: {
			icon: Send,
			label: 'sqs',
			bg: 'bg-orange-50 dark:bg-orange-900/30',
			border: 'outline-orange-300 dark:outline-orange-600/60',
			borderUnsaved: 'outline-dashed outline-orange-400',
			iconText: 'text-orange-700 dark:text-orange-400'
		},
		gcp: {
			icon: CloudCog,
			label: 'gcp',
			bg: 'bg-emerald-50 dark:bg-emerald-900/30',
			border: 'outline-emerald-300 dark:outline-emerald-600/60',
			borderUnsaved: 'outline-dashed outline-emerald-400',
			iconText: 'text-emerald-700 dark:text-emerald-400'
		}
	}
</script>

<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import { NODE } from '$lib/components/graph/util'
	import { twMerge } from 'tailwind-merge'
	import { AlertTriangle } from 'lucide-svelte'

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
			// Page-supplied dispatcher that opens the matching native
			// trigger drawer with `script_path` pre-filled. When absent
			// (e.g. webhook, schedule, or a kind without an editor) the
			// placeholder is non-clickable.
			onCreateMissingTrigger?: (kind: NativeTriggerKind, scriptPath: string) => void
		}
	}
	let { data }: Props = $props()

	let style = $derived(TRIGGER_NODE_STYLE[data.kind])
	let Icon = $derived(data.missing ? AlertTriangle : style.icon)
	let missingTitle = $derived(
		data.missing
			? `Missing ${style.label} trigger: ${data.runnable_path ?? ''} declares \`// on ${style.label}\` but no ${style.label} trigger targets it. Click to create one, or remove the annotation.`
			: undefined
	)
	// Schedule and webhook have no dedicated drawer (schedules are
	// inline-managed; webhooks are implicit endpoints), so the placeholder
	// is not clickable for those kinds.
	let canCreate = $derived(
		data.missing &&
			data.kind !== 'schedule' &&
			data.kind !== 'webhook' &&
			!!data.runnable_path &&
			!!data.onCreateMissingTrigger
	)

	function handleMissingClick() {
		if (!canCreate || !data.runnable_path || !data.onCreateMissingTrigger) return
		data.onCreateMissingTrigger(data.kind as NativeTriggerKind, data.runnable_path)
	}
</script>

<div class="relative">
	{#if canCreate}
		<!-- Whole missing placeholder is a button so the cursor + a11y
		     affordance reads "clickable to fix". Sits in the same visual
		     box as the non-clickable variants. -->
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
					Click to create
				</span>
			</div>
		</button>
	{:else}
		<div
			class={twMerge(
				'flex items-center rounded-md drop-shadow-sm overflow-hidden outline outline-1',
				data.missing
					? 'bg-red-50 dark:bg-red-900/30 outline-dashed outline-red-400 dark:outline-red-500'
					: style.bg,
				data.missing ? '' : data.unsaved ? `opacity-80 ${style.borderUnsaved}` : style.border
			)}
			style="width: {NODE.width}px; min-height: {NODE.height}px;"
			title={missingTitle ??
				(data.unsaved ? `Unsaved ${style.label}: ${data.ref}` : `${style.label}: ${data.ref}`)}
		>
			<Icon
				size={14}
				class={`shrink-0 ml-2 mr-2 ${data.missing ? 'text-red-600 dark:text-red-400' : style.iconText}`}
			/>
			<div class="flex flex-col min-w-0 flex-1 pr-2 py-0.5 leading-tight">
				<span
					class={twMerge(
						'text-3xs uppercase tracking-wide truncate',
						data.missing ? 'text-red-700 dark:text-red-400' : 'text-tertiary'
					)}
				>
					{style.label}{data.missing ? ' · missing' : data.unsaved ? ' · unsaved' : ''}
				</span>
				<span
					class={twMerge(
						'text-2xs font-mono truncate',
						data.missing ? 'text-red-700 dark:text-red-400' : 'text-emphasis'
					)}
				>
					{data.missing ? 'no trigger row' : data.ref}
				</span>
			</div>
		</div>
	{/if}
</div>

<Handle type="source" position={Position.Bottom} isConnectable={false} />
