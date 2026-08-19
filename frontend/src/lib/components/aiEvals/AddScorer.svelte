<script lang="ts">
	import { Button } from '$lib/components/common'
	import Label from '$lib/components/Label.svelte'
	import Path from '$lib/components/Path.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import AIProviderPicker from '$lib/components/AIProviderPicker.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import {
		AiEvalsService,
		ResourceService,
		ScriptService,
		type ProviderConfig,
		type Scorer
	} from '$lib/gen'
	import { loadStoredConfig } from '$lib/components/aiProviderStorage'
	import { sendUserToast } from '$lib/toast'
	import { onMount, untrack } from 'svelte'
	import { summaryToName } from '$lib/utils'
	import { Plus } from 'lucide-svelte'
	import type { RecentScorersResponse } from '$lib/gen'
	import type { ScorerKind } from './evalScorers'

	let {
		workspace,
		datasetPath,
		kind,
		mode,
		onAdd,
		onEditScript
	}: {
		workspace: string
		datasetPath: string
		kind: ScorerKind
		/** Writing a scorer and picking one that exists are different jobs with different fields,
		 *  so they are different forms rather than one with half of it below a divider. */
		mode: 'new' | 'existing'
		onAdd: (scorer: Scorer) => Promise<void>
		/** Opens the script editor on what was just created, by hash. */
		onEditScript: (hash: string) => void
	} = $props()

	/** What a scorer of this kind is called before it is called anything else. Prefilled rather
	 *  than left empty: it names the column and derives the path, so an empty one is two decisions
	 *  before the first score. */
	const DEFAULT_SUMMARY: Record<ScorerKind, string> = {
		agent: 'AI judge',
		script: 'Code scorer'
	}

	let path = $state('')
	let pathError = $state('')
	// What the column is for. It names the runnable, as a script's summary names its path, and it is
	// what the column header shows instead of the last segment of a path.
	// Seeded from the kind this form was opened for. The drawer keys the form on every open, so the
	// initial value is the whole of it — hence `untrack` rather than a derived that would fight the
	// reader's own typing.
	let summary = $state(untrack(() => DEFAULT_SUMMARY[kind]))
	let pathDirty = $state(false)
	let pathInput: Path | undefined = $state(undefined)
	$effect(() => {
		const next = summary
		untrack(() => {
			if (pathDirty || !next) return
			pathInput?.setName(`${datasetName}_${summaryToName(next)}`)
		})
	})
	/** The dataset's own name, which every scorer of it is named under. */
	let datasetName = $derived(datasetPath.split('/').pop() ?? datasetPath)
	let prompt = $state('')
	let provider = $state<ProviderConfig | undefined>(loadStoredConfig())
	let template = $state('')
	let existing = $state<string | undefined>(undefined)
	// A column is a number, and optionally a number with a line through it. The line is where the
	// column stops reporting how good an answer was and starts reporting whether it was good
	// enough, which is the question most datasets are actually asking.
	let passIf = $state('')
	let threshold = $derived(passIf && !Number.isNaN(Number(passIf)) ? Number(passIf) : undefined)
	let busy = $state(false)
	let seeded = $state(false)
	// The scorers this workspace already uses, so a new dataset does not start by retyping the
	// path of the judge you wrote last week. Filtered server-side to what the caller can read.
	let recent = $state<RecentScorersResponse>([])

	/** The run as the judge reads it, mirroring what the API renders into the user message. */
	const RUN_SHAPE = `Request: the case's user message
Prior turns: the replayed conversation, when the case has one
Tool calls, in order:
1. tool_name({"arg": ...}) -> result (123ms)
Answer: what the run produced
Expected: the case's expected value`

	let modelReady = $derived(Boolean(provider?.resource && provider?.model))
	let canCreate = $derived(
		seeded && !busy && !!path && !pathError && (kind === 'script' || (modelReady && !!prompt))
	)

	/** A scorer is created next to the dataset it scores, so it is findable from it. The suffix
	 *  only avoids the collision the prefilled name would otherwise open on. */
	async function freePath(base: string, exists: (path: string) => Promise<boolean>) {
		for (let suffix = 0; suffix < 50; suffix++) {
			const candidate = suffix === 0 ? base : `${base}_${suffix + 1}`
			if (!(await exists(candidate))) return candidate
		}
		return base
	}

	onMount(async () => {
		AiEvalsService.recentScorers({ workspace, kind })
			.then((scorers) => (recent = scorers))
			.catch(() => {
				// A list of suggestions: failing to load one is not worth a toast over the form.
			})
		try {
			const defaults = await AiEvalsService.scorerDefaults({ workspace })
			prompt = defaults.judge_prompt
			template = defaults.script_template
			path =
				kind === 'agent'
					? await freePath(`${datasetPath}_judge`, (p) =>
							ResourceService.existsResource({ workspace, path: p })
						)
					: await freePath(`${datasetPath}_scorer`, async (p) =>
							Boolean(await ScriptService.existsScriptByPath({ workspace, path: p }))
						)
		} catch (e) {
			sendUserToast(`Failed to load the scorer defaults: ${e}`, true)
		} finally {
			seeded = true
		}
	})

	async function createJudge() {
		await ResourceService.createResource({
			workspace,
			requestBody: {
				path,
				resource_type: 'ai_agent',
				description: summary || `Judge for eval dataset ${datasetPath}`,
				value: { provider, system_prompt: prompt, output_type: 'text' }
			}
		})
		await onAdd({ kind: 'agent', path, name: summary || undefined, pass_if: threshold })
	}

	async function createScript() {
		const hash = await ScriptService.createScript({
			workspace,
			requestBody: {
				path,
				summary: summary || `Scorer for ${datasetPath}`,
				description: '',
				content: template,
				language: 'bun'
			}
		})
		await onAdd({ kind: 'script', path, name: summary || undefined, pass_if: threshold })
		// The template is a starting point, so the editor opens on it: writing the assertions is
		// the actual work, and it happens over the table rather than after hunting for the file.
		onEditScript(hash)
	}

	async function create() {
		busy = true
		try {
			if (kind === 'agent') {
				await createJudge()
			} else {
				await createScript()
			}
		} catch (e) {
			sendUserToast(`Failed to create the scorer: ${e}`, true)
		} finally {
			busy = false
		}
	}
</script>

<div class="flex flex-col gap-6">
	{#if mode === 'new'}
	{#if kind === 'agent'}
		<span class="text-xs text-secondary">
			An agent handed one whole run to grade. It is an ordinary AI agent resource: this creates it
			with the prompt below, and editing the column later means editing that agent.
		</span>
	{:else}
		<span class="text-xs text-secondary">
			A script handed the same run, returning a number, a boolean or {'{ score, reason, checks }'}.
			The template puts the assertions in main and the helpers below it: exact match, tool called
			and not called, arguments against each tool's schema, repeated calls, step errors, latency.
		</span>
	{/if}

	<Label label="Summary">
		<TextInput
			bind:value={summary}
			size="sm"
			inputProps={{
				placeholder: kind === 'agent' ? 'Answers the question asked' : 'Tool discipline'
			}}
		/>
	</Label>

	<Label label="Path">
		<Path
			bind:this={pathInput}
			bind:path
			bind:error={pathError}
			bind:dirty={pathDirty}
			initialPath=""
			namePlaceholder={kind === 'agent' ? 'judge' : 'scorer'}
			kind={kind === 'agent' ? 'resource' : 'script'}
			workspaceOverride={workspace}
			autofocus={false}
			size="sm"
		/>
	</Label>

	<Label
		label="Pass threshold"
		tooltip="Optional. A case scoring at or above this counts as a pass, and the column reports a pass rate beside its mean. It can be set or changed later from the column header, and reads the scores already recorded."
	>
		<TextInput bind:value={passIf} size="sm" inputProps={{ placeholder: '0.7' }} />
	</Label>

	{#if kind === 'agent'}
		<Label label="Model">
			<AIProviderPicker bind:value={provider} />
		</Label>

		<Label
			label="Grading prompt"
			tooltip="The judge's system prompt. It is stored on the agent, so it can be rewritten later without touching the dataset."
		>
			<TextInput
				underlyingInputEl="textarea"
				size="sm"
				unifiedHeight={false}
				class="min-h-56 font-mono !text-2xs"
				bind:value={prompt}
				inputProps={{ spellcheck: false }}
			/>
		</Label>

		<Label
			label="What the judge is sent"
			tooltip="The run is passed as the agent's user message, through the same input transform an AI agent step uses."
		>
			<pre
				class="text-2xs text-secondary bg-surface-secondary rounded-md p-3 overflow-x-auto whitespace-pre"
				>{RUN_SHAPE}</pre
			>
		</Label>
	{/if}

	<div class="flex justify-end">
		<Button
			size="xs"
			variant="accent"
			startIcon={{ icon: Plus }}
			disabled={!canCreate}
			onclick={create}
		>
			{#if kind === 'agent'}
				{modelReady ? 'Create judge agent' : 'Pick a model'}
			{:else}
				Create script and open it
			{/if}
		</Button>
	</div>

	{:else}
	{#if recent.length > 0}
		<Label label="Reuse one from another dataset">
			<div class="flex flex-col divide-y border rounded-md">
				{#each recent as scorer (scorer.path)}
					<button
						type="button"
						class="flex items-center gap-2 px-3 py-2 text-left hover:bg-surface-hover disabled:opacity-50"
						disabled={busy}
						onclick={() =>
							onAdd({
								kind: scorer.kind,
								path: scorer.path,
								name: scorer.name,
								pass_if: threshold ?? scorer.pass_if
							})}
					>
						<span class="text-xs text-emphasis truncate">{scorer.name || scorer.path}</span>
						<span class="text-2xs text-tertiary truncate">{scorer.path}</span>
						<div class="grow"></div>
						<span class="text-2xs text-tertiary truncate">{scorer.dataset}</span>
					</button>
				{/each}
			</div>
		</Label>
	{/if}

	<Label label={kind === 'agent' ? 'An agent in this workspace' : 'A script in this workspace'}>
		<div class="flex items-center gap-2">
			<div class="grow min-w-0">
				{#if kind === 'agent'}
					<ResourcePicker bind:value={existing} resourceType="ai_agent" />
				{:else}
					<ScriptPicker bind:scriptPath={existing} kinds={['script']} clearable {workspace} />
				{/if}
			</div>
			<Button
				size="xs"
				variant="default"
				disabled={busy || !existing}
				onclick={() =>
					existing &&
					onAdd({ kind, path: existing, name: summary || undefined, pass_if: threshold })}
			>
				Add
			</Button>
		</div>
	</Label>
	{/if}
</div>
