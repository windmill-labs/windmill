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
		CloudCog
	} from 'lucide-svelte'
	import type { ScriptLang } from '$lib/gen'
	import type { NativeTriggerKind } from './types'

	// Each left-column kind is just "materializer triggered by <trigger
	// source>". id === the SCRIPT_TRIGGER_KIND value, so the handler can
	// dispatch on it uniformly. Asset-triggered materializers are not in
	// this menu; those live under the per-asset + inside the graph.
	type KindId = 'schedule' | NativeTriggerKind

	interface Props {
		data: {
			onAddMaterializer: (
				language: ScriptLang,
				path: string,
				source:
					| { kind: 'schedule'; cron: string }
					| { kind: NativeTriggerKind; path: string | undefined }
			) => void
			pathPrefix: string
			defaultPathSuffix: string
			defaultScheduleCron: string
		}
	}
	let { data }: Props = $props()

	const LANGUAGES: Array<{ label: string; lang: ScriptLang }> = [
		{ label: 'TypeScript (Bun)', lang: 'bun' },
		{ label: 'TypeScript (Deno)', lang: 'deno' },
		{ label: 'Python', lang: 'python3' },
		{ label: 'PostgreSQL', lang: 'postgresql' },
		{ label: 'DuckDB', lang: 'duckdb' },
		{ label: 'BigQuery', lang: 'bigquery' },
		{ label: 'Snowflake', lang: 'snowflake' },
		{ label: 'MySQL', lang: 'mysql' },
		{ label: 'MS SQL', lang: 'mssql' },
		{ label: 'Bash', lang: 'bash' },
		{ label: 'Go', lang: 'go' }
	]

	function handlePick(pick: PipelineInsertPick) {
		if (!pick.language || !pick.path) return
		const kindId = pick.kindId as KindId
		if (kindId === 'schedule') {
			data.onAddMaterializer(pick.language as ScriptLang, pick.path, {
				kind: 'schedule',
				cron: data.defaultScheduleCron
			})
		} else {
			// Native trigger reference: user is expected to fill in the
			// trigger path themselves in the editor (or configure it in the
			// trigger's own UI). We seed the annotation with an empty ref
			// the user replaces.
			data.onAddMaterializer(pick.language as ScriptLang, pick.path, {
				kind: kindId,
				path: undefined
			})
		}
	}
</script>

<PipelineInsertMenu
	kinds={[
		{
			id: 'schedule',
			label: 'On schedule',
			description: 'Cron-driven materializer',
			icon: Clock,
			pickLanguage: true
		},
		{
			id: 'webhook',
			label: 'On webhook',
			description: 'Triggered by an HTTP webhook',
			icon: Webhook,
			pickLanguage: true
		},
		{
			id: 'email',
			label: 'On email',
			description: 'Triggered by incoming email',
			icon: Mail,
			pickLanguage: true
		},
		{
			id: 'kafka',
			label: 'On Kafka',
			description: 'Triggered by a Kafka message',
			icon: Zap,
			pickLanguage: true
		},
		{
			id: 'mqtt',
			label: 'On MQTT',
			description: 'Triggered by an MQTT message',
			icon: Radio,
			pickLanguage: true
		},
		{
			id: 'nats',
			label: 'On NATS',
			description: 'Triggered by a NATS message',
			icon: MessageSquare,
			pickLanguage: true
		},
		{
			id: 'postgres',
			label: 'On Postgres',
			description: 'Triggered by a Postgres event',
			icon: Database,
			pickLanguage: true
		},
		{
			id: 'sqs',
			label: 'On SQS',
			description: 'Triggered by an SQS message',
			icon: Send,
			pickLanguage: true
		},
		{
			id: 'gcp',
			label: 'On GCP Pub/Sub',
			description: 'Triggered by a Pub/Sub message',
			icon: CloudCog,
			pickLanguage: true
		}
	]}
	languages={LANGUAGES as any}
	pathPrefix={data.pathPrefix}
	defaultPathSuffix={data.defaultPathSuffix}
	onPick={handlePick}
>
	{#snippet trigger()}
		<button
			type="button"
			class="w-10 h-10 rounded-full flex items-center justify-center bg-emerald-500 hover:bg-emerald-600 text-white shadow-md transition-colors cursor-pointer"
			title="Add to pipeline"
		>
			<Plus size={20} />
		</button>
	{/snippet}
</PipelineInsertMenu>
