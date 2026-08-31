<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import Path from '$lib/components/Path.svelte'
	import { Button } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import AIProviderPicker from '$lib/components/AIProviderPicker.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import {
		AiEvalsService,
		ResourceService,
		ScriptService,
		type EvalDataset,
		type ProviderConfig,
		type Scorer
	} from '$lib/gen'
	import { loadStoredConfig } from '$lib/components/aiProviderStorage'
	import { sendUserToast } from '$lib/toast'
	import { onMount, untrack } from 'svelte'
	import { Bot, Code2 } from 'lucide-svelte'
	import type { RecentScorersResponse } from '$lib/gen'
	import { datasetSummary, parseThreshold, summaryToName, type ScorerKind } from './evalUtils'

	let {
		workspace,
		datasetPath,
		datasets,
		kind,
		mode,
		onAdd,
		onEditScript
	}: {
		workspace: string
		datasetPath: string
		/** The workspace's datasets, for naming the one a reusable scorer already measures. */
		datasets: EvalDataset[]
		kind: ScorerKind
		mode: 'new' | 'existing'
		onAdd: (scorer: Scorer) => Promise<void>
		/** Opens the script editor on what was just created, by hash. */
		onEditScript: (hash: string) => void
	} = $props()

	/** Prefilled rather than left empty: it names the column and derives the path. */
	const DEFAULT_SUMMARY: Record<ScorerKind, string> = {
		agent: 'AI judge',
		script: 'Code scorer'
	}

	let path = $state('')
	let pathError = $state('')
	// Seeded from the kind, and only seeded: the drawer keys the form on every open, so the initial
	// value is the whole of it, where a derived would fight the reader's own typing.
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
	let passIf = $state('')
	let threshold = $derived(parseThreshold(passIf))
	let busy = $state(false)
	let seeded = $state(false)
	/** The scorers this workspace already uses, filtered server-side to what the caller can read. */
	let recent = $state<RecentScorersResponse>([])
	/** Which list to pick from. Starts on the scorers already measuring something. */
	let source = $state<'recent' | 'any'>('recent')
	/** The row picked out of that list. Picking is not adding: the drawer's own button adds. */
	let picked = $state<RecentScorersResponse[number] | undefined>(undefined)
	let usingRecent = $derived(source === 'recent' && recent.length > 0)

	/** The run as the judge reads it, mirroring what the API renders into the user message. */
	const RUN_SHAPE = `Request: the case's user message
Tool calls, in order:
1. tool_name({"arg": ...}) -> result (123ms)
Answer: what the run produced
Expected: the case's expected value`

	let modelReady = $derived(Boolean(provider?.resource && provider?.model))
	let canCreate = $derived(
		seeded &&
			!busy &&
			!!path &&
			!pathError &&
			!threshold.error &&
			(kind === 'script' || (modelReady && !!prompt))
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

	/**
	 * The shape a verdict has to arrive in. Asking for it in the prompt leaves the model free to
	 * quote the agent inside its own reason and break the JSON around it; an output schema is the
	 * provider enforcing the shape instead.
	 */
	const VERDICT_SCHEMA = {
		$schema: 'https://json-schema.org/draft/2020-12/schema',
		type: 'object',
		properties: {
			score: { type: 'number', description: 'How well the agent did, from 0 to 1.' },
			reason: { type: 'string', description: 'One or two sentences on why.' }
		},
		required: ['score'],
		order: ['score', 'reason']
	}

	async function createJudge() {
		await ResourceService.createResource({
			workspace,
			requestBody: {
				path,
				resource_type: 'ai_agent',
				description: summary || `Judge for eval dataset ${datasetPath}`,
				value: {
					provider,
					system_prompt: prompt,
					output_type: 'text',
					output_schema: VERDICT_SCHEMA
				}
			}
		})
		await onAdd({ kind: 'agent', path, name: summary || undefined, pass_if: threshold.value })
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
		await onAdd({ kind: 'script', path, name: summary || undefined, pass_if: threshold.value })
		onEditScript(hash)
	}

	async function create() {
		if (kind === 'agent') {
			await createJudge()
		} else {
			await createScript()
		}
	}

	/** Adds the scorer picked out of one of the two lists, as this dataset's column. */
	async function addExisting() {
		if (usingRecent) {
			if (!picked) return
			await onAdd({
				kind: picked.kind,
				path: picked.path,
				name: picked.name,
				pass_if: threshold.value ?? picked.pass_if
			})
			return
		}
		if (!existing) return
		await onAdd({ kind, path: existing, name: summary || undefined, pass_if: threshold.value })
	}

	/** The drawer's own button drives the form, so what it says and whether it can be pressed are
	 *  read from here. */
	export function submitState(): {
		label: string
		disabled: boolean
		busy: boolean
		title?: string
	} {
		// What is still missing, since the button sits at the top of a form that runs past it.
		if (mode === 'new') {
			const missing = !modelReady ? 'Pick a model first' : !prompt ? 'Write a prompt first' : ''
			return {
				label: kind === 'agent' ? 'Create judge' : 'Create and open',
				disabled: !canCreate,
				busy,
				title: canCreate || kind === 'script' ? undefined : missing || undefined
			}
		}
		const chosen = usingRecent ? picked : existing
		return {
			label: 'Add scorer',
			disabled: busy || !chosen,
			busy,
			title: chosen ? undefined : usingRecent ? 'Pick a scorer from the list' : 'Pick one first'
		}
	}

	export async function submit() {
		busy = true
		try {
			if (mode === 'new') {
				await create()
			} else {
				await addExisting()
			}
		} catch (e) {
			sendUserToast(`Failed to add the scorer: ${e}`, true)
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
				The template scores the answer against the case's expected one, reports how the agent got
				there as checks beside it, and leaves a case with no expected answer unmeasured. Helpers
				below it cover exact and structural matches, which tools were called, arguments against each
				tool's schema, repeated calls, step errors, latency and cost.
			</span>
		{/if}

		<Label label="Summary">
			<TextInput
				bind:value={summary}
				size="sm"
				inputProps={{
					maxlength: 120,
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
			<TextInput
				bind:value={passIf}
				size="sm"
				error={threshold.error}
				inputProps={{ placeholder: '0.7', type: 'number', min: 0, max: 1, step: 0.05 }}
			/>
		</Label>

		{#if kind === 'agent'}
			<Label label="Model">
				<AIProviderPicker bind:value={provider} />
			</Label>

			<Label
				label="Grading prompt"
				tooltip="Stored on the agent, so it can be rewritten later without touching the dataset."
			>
				<span class="text-xs text-secondary">
					The judge's system prompt: how to score, written once for every case.
				</span>
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
				tooltip="Passed through the same input transform an AI agent step uses."
			>
				<span class="text-xs text-secondary">
					One run per message, in this shape. The prompt above is what reads it.
				</span>
				<pre
					class="text-2xs text-secondary bg-surface-secondary rounded-md p-3 overflow-x-auto whitespace-pre"
					>{RUN_SHAPE}</pre
				>
			</Label>
		{/if}
	{:else}
		{#if recent.length > 0}
			<ToggleButtonGroup bind:selected={source} class="w-fit">
				{#snippet children({ item })}
					<ToggleButton
						value="recent"
						label="Scorer"
						tooltip="Already measuring another dataset of this workspace."
						{item}
					/>
					<ToggleButton
						value="any"
						label={kind === 'agent' ? 'Any agent' : 'Any script'}
						tooltip="Anything in the workspace, whether it has scored before or not."
						{item}
					/>
				{/snippet}
			</ToggleButtonGroup>
		{/if}

		{#if usingRecent}
			<div class="flex flex-col divide-y border rounded-md">
				{#each recent as scorer (scorer.path)}
					{@const measures = datasetSummary(datasets, scorer.dataset)}
					<Button
						variant="subtle"
						unifiedSize="sm"
						disabled={busy}
						wrapperClasses="w-full"
						btnClasses="w-full !h-auto !justify-start !rounded-none flex items-center gap-3 px-3 py-2 text-left !font-normal {picked?.path ===
						scorer.path
							? 'bg-blue-50 dark:bg-blue-900/50 hover:bg-blue-50 dark:hover:bg-blue-900/50'
							: 'hover:bg-surface-hover'}"
						onClick={() => (picked = scorer)}
					>
						{#if scorer.kind === 'agent'}
							<Bot size={14} class="text-tertiary shrink-0" />
						{:else}
							<Code2 size={14} class="text-tertiary shrink-0" />
						{/if}
						<div class="flex flex-col min-w-0 grow">
							<span class="text-xs text-emphasis truncate leading-tight">
								{scorer.name || scorer.path}
							</span>
							{#if scorer.name}
								<span class="text-2xs text-tertiary truncate leading-tight">{scorer.path}</span>
							{/if}
						</div>
						<span
							class="flex items-center gap-1.5 text-2xs text-tertiary min-w-0 shrink"
							title={scorer.dataset}
						>
							<span class="flex flex-col min-w-0 text-right">
								{#if measures}
									<span class="truncate leading-tight">{measures}</span>
								{/if}
								<span class="truncate leading-tight">{scorer.dataset}</span>
							</span>
						</span>
					</Button>
				{/each}
			</div>
		{:else if kind === 'agent'}
			<ResourcePicker bind:value={existing} resourceType="ai_agent" {workspace} />
		{:else}
			<ScriptPicker bind:scriptPath={existing} kinds={['script']} clearable {workspace} />
		{/if}
	{/if}
</div>
