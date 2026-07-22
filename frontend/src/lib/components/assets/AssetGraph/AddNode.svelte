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
				aiPrompt?: string
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
			pick.aiPrompt
		)
	}
</script>

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
			label: 'On email',
			description: 'Triggered by incoming email',
			icon: Mail
		},
		{
			id: 'kafka',
			label: 'On Kafka',
			description: 'Triggered by a Kafka message',
			icon: Zap
		},
		{
			id: 'mqtt',
			label: 'On MQTT',
			description: 'Triggered by an MQTT message',
			icon: Radio
		},
		{
			id: 'amqp',
			label: 'On AMQP',
			description: 'Triggered by an AMQP (RabbitMQ) message',
			icon: Radio
		},
		{
			id: 'nats',
			label: 'On NATS',
			description: 'Triggered by a NATS message',
			icon: MessageSquare
		},
		{
			id: 'postgres',
			label: 'On Postgres',
			description: 'Triggered by a Postgres event',
			icon: Database
		},
		{
			id: 'sqs',
			label: 'On SQS',
			description: 'Triggered by an SQS message',
			icon: Send
		},
		{
			id: 'gcp',
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
		     accent circle outweighed every real node on the canvas. -->
		<button
			type="button"
			class="w-8 h-8 rounded-full flex items-center justify-center bg-surface border border-gray-400 dark:border-gray-600 text-secondary shadow-sm hover:bg-surface-hover transition-colors cursor-pointer"
			title="Add to pipeline"
		>
			<Plus size={16} />
		</button>
	{/snippet}
</PipelineInsertMenu>
