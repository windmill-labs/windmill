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

	interface Props {
		// `ref` is the cron expression for schedules, the trigger-path for
		// every other kind. Rendered verbatim — no formatting per kind.
		data: { kind: TriggerNodeKind; ref: string; unsaved?: boolean }
	}
	let { data }: Props = $props()

	let style = $derived(TRIGGER_NODE_STYLE[data.kind])
	let Icon = $derived(style.icon)
</script>

<div class="relative">
	<div
		class={twMerge(
			'flex items-center rounded-md drop-shadow-sm overflow-hidden outline outline-1',
			style.bg,
			data.unsaved ? `opacity-80 ${style.borderUnsaved}` : style.border
		)}
		style="width: {NODE.width}px; min-height: {NODE.height}px;"
		title={data.unsaved ? `Unsaved ${style.label}: ${data.ref}` : `${style.label}: ${data.ref}`}
	>
		<Icon size={14} class={`shrink-0 ml-2 mr-2 ${style.iconText}`} />
		<div class="flex flex-col min-w-0 flex-1 pr-2 py-0.5 leading-tight">
			<span class="text-3xs uppercase tracking-wide text-tertiary truncate">
				{style.label}{data.unsaved ? ' · unsaved' : ''}
			</span>
			<span class="text-2xs font-mono text-emphasis truncate">{data.ref}</span>
		</div>
	</div>
</div>

<Handle type="source" position={Position.Bottom} isConnectable={false} />
