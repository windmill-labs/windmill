<!--
@component
The menu that adds a pipeline script started by a data source (a schedule, a
webhook, a queue…): the add node's, and the canvas's right-click one.
-->
<script lang="ts">
	import type { Snippet } from 'svelte'
	import PipelineInsertMenu, {
		type PipelineInsertOptions,
		type PipelineInsertPick
	} from './PipelineInsertMenu.svelte'
	import {
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

	interface Props {
		onAddPipelineScript: (
			language: ScriptLang,
			path: string,
			source: { kind: NativeTriggerKind; path: string | undefined },
			outputKind: PipelineOutputKind,
			aiPrompt?: string,
			options?: PipelineInsertOptions
		) => void
		pathPrefix: string
		defaultPathSuffix: string
		trigger: Snippet<[{ open: boolean }]>
		/** Opens the menu each time it changes. */
		openSignal?: number
	}
	let { onAddPipelineScript, pathPrefix, defaultPathSuffix, trigger, openSignal }: Props =
		$props()

	function handlePick(pick: PipelineInsertPick) {
		if (!pick.language || !pick.path) return
		// Native trigger annotation is marker-only — the binding lives on the
		// trigger row's own `script_path`, which is created separately.
		onAddPipelineScript(
			pick.language as ScriptLang,
			pick.path,
			{ kind: pick.kindId as NativeTriggerKind, path: undefined },
			(pick.outputKind ?? 'none') as PipelineOutputKind,
			pick.aiPrompt,
			{ schedule: pick.schedule, outputAsset: pick.outputAsset }
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
	{pathPrefix}
	{defaultPathSuffix}
	onPick={handlePick}
	{openSignal}
	{trigger}
/>
