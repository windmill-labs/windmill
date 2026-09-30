<script lang="ts">
	import PipelineInsertMenu, { type PipelineInsertPick } from './PipelineInsertMenu.svelte'
	import {
		Plus,
		Clock,
		Webhook,
		Mail,
		Zap,
		Radio,
		MessageSquare,
		Database,
		Send,
		CloudCog,
		Upload
	} from 'lucide-svelte'
	import type { ScriptLang } from '$lib/gen'
	import type { NativeTriggerKind } from './types'
	import { PIPELINE_LANGUAGES } from './pipelineLanguages'
	import { NODE } from '$lib/components/graph/util'
	import type { PipelineOutputKind } from './pipelineTemplates'

	// Each left-column kind is just "pipeline script triggered by <trigger
	// source>". id === the SCRIPT_TRIGGER_KIND value, so the handler can
	// dispatch on it uniformly. Asset-triggered scripts are not in this
	// menu; those live under the per-asset + inside the graph.
	type KindId = NativeTriggerKind

	interface Props {
		data: {
			onAddPipelineScript: (
				language: ScriptLang,
				path: string,
				source: { kind: NativeTriggerKind; path: string | undefined },
				outputKind: PipelineOutputKind,
				aiPrompt?: string,
				options?: import('./PipelineInsertMenu.svelte').PipelineInsertOptions
			) => void
			pathPrefix: string
			defaultPathSuffix: string
		}
	}
	let { data }: Props = $props()

	function handlePick(pick: PipelineInsertPick) {
		if (!pick.language || !pick.path) return
		const kindId = pick.kindId as KindId
		const outputKind = (pick.outputKind ?? 'none') as PipelineOutputKind
		// Native trigger annotation is marker-only — the binding lives on
		// the trigger row's own `script_path`, which the user creates
		// separately. Seed with `path: undefined`.
		data.onAddPipelineScript(
			pick.language as ScriptLang,
			pick.path,
			{ kind: kindId, path: undefined },
			outputKind,
			pick.aiPrompt,
			{ schedule: pick.schedule, outputAsset: pick.outputAsset }
		)
	}
</script>

<!-- The layout gives this node a full node-width slot; centering the pill in
     it lines it up with the nodes below whatever its label's width. -->
<div class="flex justify-center" style="width: {NODE.width}px;">
<PipelineInsertMenu
	kinds={[
		{
			id: 'data_upload',
			label: 'On data upload',
			description: 'UI-first: run by uploading a file via the S3 picker',
			icon: Upload
		},
		{
			id: 'schedule',
			label: 'On schedule',
			description: 'Triggered by a schedule you create',
			icon: Clock
		},
		{
			id: 'webhook',
			label: 'On webhook',
			description: 'Triggered by an HTTP webhook',
			icon: Webhook
		},
		{
			id: 'email',
			configuredAfterCreate: true,
			label: 'On email',
			description: 'Triggered by incoming email',
			icon: Mail
		},
		{
			id: 'kafka',
			configuredAfterCreate: true,
			label: 'On Kafka',
			description: 'Triggered by a Kafka message',
			icon: Zap
		},
		{
			id: 'mqtt',
			configuredAfterCreate: true,
			label: 'On MQTT',
			description: 'Triggered by an MQTT message',
			icon: Radio
		},
		{
			id: 'amqp',
			configuredAfterCreate: true,
			label: 'On AMQP',
			description: 'Triggered by an AMQP (RabbitMQ) message',
			icon: Radio
		},
		{
			id: 'nats',
			configuredAfterCreate: true,
			label: 'On NATS',
			description: 'Triggered by a NATS message',
			icon: MessageSquare
		},
		{
			id: 'postgres',
			configuredAfterCreate: true,
			label: 'On Postgres',
			description: 'Triggered by a Postgres event',
			icon: Database
		},
		{
			id: 'sqs',
			configuredAfterCreate: true,
			label: 'On SQS',
			description: 'Triggered by an SQS message',
			icon: Send
		},
		{
			id: 'gcp',
			configuredAfterCreate: true,
			label: 'On GCP Pub/Sub',
			description: 'Triggered by a Pub/Sub message',
			icon: CloudCog
		}
	]}
	languages={PIPELINE_LANGUAGES as any}
	pathPrefix={data.pathPrefix}
	defaultPathSuffix={data.defaultPathSuffix}
	onPick={handlePick}
>
	{#snippet trigger()}
		<!-- Quiet insert affordance, mirroring the flow editor's inline +
		     buttons (bg-surface + gray border + secondary text) — a filled
		     accent pill would outweigh every real node on the canvas. -->
		<button
			type="button"
			class="h-8 px-3 rounded-full flex items-center gap-1.5 whitespace-nowrap bg-surface border border-gray-400 dark:border-gray-600 text-xs font-normal text-secondary shadow-sm hover:bg-surface-hover transition-colors cursor-pointer"
		>
			<Plus size={16} class="shrink-0" />
			Add data source
		</button>
	{/snippet}
</PipelineInsertMenu>
</div>
