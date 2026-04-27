<script lang="ts">
	import { base } from '$lib/base'
	import { goto } from '$app/navigation'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import type { ScriptLang } from '$lib/gen'
	import { Code2 } from 'lucide-svelte'

	interface Props {
		open: boolean
		folder: string
		// When present, an `// on <asset>` trigger line is prefilled below the
		// `// materialize` marker. Used by the per-asset + on an asset node so
		// the new script is wired to refresh when that asset changes.
		triggerAsset?: { prefix: string; path: string } | undefined
		onOpenChange: (v: boolean) => void
	}
	let { open = $bindable(), folder, triggerAsset = undefined, onOpenChange }: Props = $props()

	const LANGUAGES: Array<{ label: string; value: ScriptLang }> = [
		{ label: 'TypeScript (Bun)', value: 'bun' },
		{ label: 'TypeScript (Deno)', value: 'deno' },
		{ label: 'Python', value: 'python3' },
		{ label: 'PostgreSQL', value: 'postgresql' },
		{ label: 'DuckDB', value: 'duckdb' },
		{ label: 'BigQuery', value: 'bigquery' },
		{ label: 'Snowflake', value: 'snowflake' },
		{ label: 'MySQL', value: 'mysql' },
		{ label: 'MS SQL', value: 'mssql' },
		{ label: 'Bash', value: 'bash' },
		{ label: 'Go', value: 'go' }
	]

	let scriptPath = $state('')
	let language = $state<ScriptLang>('bun')

	$effect(() => {
		if (open) {
			scriptPath = `f/${folder}/new_materializer`
			language = 'bun'
		}
	})

	let canConfirm = $derived(
		scriptPath.trim().startsWith(`f/${folder}/`) && scriptPath.trim().length > `f/${folder}/`.length
	)

	function confirm() {
		if (!canConfirm) return
		const params = new URLSearchParams({
			path: scriptPath.trim(),
			lang: language,
			materialize: '1'
		})
		if (triggerAsset) {
			params.set('on_asset', `${triggerAsset.prefix}${triggerAsset.path}`)
		}
		onOpenChange(false)
		goto(`${base}/scripts/add?${params}`)
	}
</script>

<Modal bind:open title={triggerAsset ? 'Add script triggered by asset' : 'Add materializer script'}>
	<div class="flex flex-col gap-4 w-full min-w-0">
		<label class="flex flex-col gap-1">
			<span class="text-xs font-medium text-secondary">Path</span>
			<TextInput bind:value={scriptPath} placeholder="f/{folder}/new_materializer" />
			<span class="text-2xs text-tertiary">
				Must live in <code>f/{folder}/</code>.
			</span>
		</label>

		<label class="flex flex-col gap-1">
			<span class="text-xs font-medium text-secondary">Language</span>
			<Select items={LANGUAGES} bind:value={language} />
		</label>

		{#if triggerAsset}
			<div class="rounded-md bg-surface-secondary border px-3 py-2 text-xs text-secondary">
				The new script will include <code>// on {triggerAsset.prefix}{triggerAsset.path}</code> so it
				refreshes when this asset changes.
			</div>
		{:else}
			<div class="rounded-md bg-surface-secondary border px-3 py-2 text-xs text-secondary">
				The new script will include a bare <code>// materialize</code> marker so it's counted as a pipeline
				member. Any writes the parser detects become its outputs.
			</div>
		{/if}
	</div>

	{#snippet actions()}
		<Button variant="subtle" unifiedSize="sm" onclick={() => onOpenChange(false)}>Cancel</Button>
		<Button
			variant="accent"
			unifiedSize="sm"
			disabled={!canConfirm}
			onclick={confirm}
			startIcon={{ icon: Code2 }}
		>
			Create script
		</Button>
	{/snippet}
</Modal>
