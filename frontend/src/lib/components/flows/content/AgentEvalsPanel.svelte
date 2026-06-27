<script lang="ts">
	import { Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import { AiAgentService, ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { randomUUID } from '$lib/utils/uuid'
	import { Play, Plus, Save, Trash2, Loader2, Check, X, ChevronDown, ChevronRight } from 'lucide-svelte'
	import type {
		AgentAssertion,
		AgentEvalCase,
		AIAgentConfig
	} from '../agentResourceUtils'

	let { agent }: { agent: string } = $props()

	type AssertionResult = { assertion: AgentAssertion; passed: boolean; detail?: string }
	type JudgeResult = { score: number; pass: boolean; summary: string }
	type CaseResult = {
		case_id: string
		passed: boolean
		output?: unknown
		error?: string
		assertions?: AssertionResult[]
		judge?: JudgeResult
		latency_ms: number
	}

	let resourceValue: AIAgentConfig = $state({})
	let cases: AgentEvalCase[] = $state([])
	let results: Record<string, CaseResult | 'running'> = $state({})
	let expanded: Record<string, boolean> = $state({})
	let loading = $state(true)
	let saving = $state(false)

	const ASSERTION_KINDS = [
		{ label: 'contains', value: 'contains' },
		{ label: 'does not contain', value: 'not_contains' },
		{ label: 'matches regex', value: 'regex' },
		{ label: 'json path equals', value: 'json_path_equals' },
		{ label: 'valid output schema', value: 'output_schema_valid' }
	]

	async function load() {
		if (!$workspaceStore) return
		loading = true
		try {
			const res = await ResourceService.getResource({ workspace: $workspaceStore, path: agent })
			resourceValue = (res.value ?? {}) as AIAgentConfig
			cases = resourceValue.evals?.cases ?? []
		} catch (e) {
			sendUserToast(`Failed to load agent: ${e}`, true)
		} finally {
			loading = false
		}
	}

	$effect(() => {
		agent && $workspaceStore && load()
	})

	function addCase() {
		cases = [
			...cases,
			{ id: randomUUID(), name: `Case ${cases.length + 1}`, input: { user_message: '' }, judge_checklist: [], assertions: [] }
		]
	}

	function removeCase(id: string) {
		cases = cases.filter((c) => c.id !== id)
		delete results[id]
	}

	function checklistText(c: AgentEvalCase): string {
		return (c.judge_checklist ?? []).join('\n')
	}
	function setChecklist(c: AgentEvalCase, text: string) {
		c.judge_checklist = text
			.split('\n')
			.map((l) => l.trim())
			.filter(Boolean)
	}

	let newAssertionKind: Record<string, string> = $state({})
	function addAssertion(c: AgentEvalCase) {
		const kind = newAssertionKind[c.id] ?? 'contains'
		let a: AgentAssertion
		if (kind === 'regex') a = { kind: 'regex', pattern: '' }
		else if (kind === 'json_path_equals') a = { kind: 'json_path_equals', path: '', value: '' }
		else if (kind === 'output_schema_valid') a = { kind: 'output_schema_valid' }
		else if (kind === 'not_contains') a = { kind: 'not_contains', value: '' }
		else a = { kind: 'contains', value: '' }
		c.assertions = [...(c.assertions ?? []), a]
	}
	function removeAssertion(c: AgentEvalCase, i: number) {
		c.assertions = (c.assertions ?? []).filter((_, idx) => idx !== i)
	}

	async function save() {
		if (!$workspaceStore) return
		saving = true
		try {
			const value = { ...resourceValue, evals: { ...(resourceValue.evals ?? {}), cases } }
			await ResourceService.updateResource({
				workspace: $workspaceStore,
				path: agent,
				requestBody: { value }
			})
			resourceValue = value
			sendUserToast('Saved eval cases')
		} catch (e) {
			sendUserToast(`Failed to save: ${e}`, true)
		} finally {
			saving = false
		}
	}

	async function runCase(c: AgentEvalCase) {
		if (!$workspaceStore) return
		results[c.id] = 'running'
		try {
			const resp = await AiAgentService.evalAiAgentCase({
				workspace: $workspaceStore,
				requestBody: { agent, case: c as unknown as Record<string, unknown> }
			})
			results[c.id] = resp as unknown as CaseResult
		} catch (e) {
			results[c.id] = {
				case_id: c.id,
				passed: false,
				error: String(e),
				latency_ms: 0
			}
		}
	}

	async function runAll() {
		// Save first so the linked agent reflects the latest cases, then run sequentially.
		await save()
		for (const c of cases) {
			await runCase(c)
		}
	}

	function outputText(output: unknown): string {
		if (typeof output === 'string') return output
		return JSON.stringify(output, null, 2)
	}

	let passedCount = $derived(
		cases.filter((c) => {
			const r = results[c.id]
			return r && r !== 'running' && r.passed
		}).length
	)
	let ranCount = $derived(
		cases.filter((c) => {
			const r = results[c.id]
			return r && r !== 'running'
		}).length
	)
</script>

<div class="flex flex-col gap-3 p-3">
	<div class="flex items-center gap-2">
		<span class="text-sm font-semibold">Evals</span>
		{#if ranCount > 0}
			<span class="text-xs text-secondary">{passedCount}/{ranCount} passed</span>
		{/if}
		<div class="ml-auto flex items-center gap-1">
			<Button size="xs2" variant="default" startIcon={{ icon: Plus }} onclick={addCase}>
				Add case
			</Button>
			<Button
				size="xs2"
				variant="default"
				startIcon={{ icon: Save }}
				disabled={saving}
				onclick={save}
			>
				Save
			</Button>
			<Button
				size="xs2"
				variant="accent"
				startIcon={{ icon: Play }}
				disabled={cases.length === 0}
				onclick={runAll}
			>
				Run all
			</Button>
		</div>
	</div>

	{#if loading}
		<div class="flex items-center gap-2 text-xs text-secondary">
			<Loader2 size={14} class="animate-spin" /> Loading…
		</div>
	{:else if cases.length === 0}
		<p class="text-xs text-secondary">
			No eval cases yet. Add a case with an input message, then grade it with a judge checklist
			and/or deterministic assertions.
		</p>
	{:else}
		{#each cases as c (c.id)}
			{@const r = results[c.id]}
			<div class="rounded-md border border-border p-3 flex flex-col gap-2">
				<div class="flex items-center gap-2">
					<input
						class="text-xs font-medium grow border border-border-light rounded px-2 py-1 bg-surface-input"
						bind:value={c.name}
						placeholder="Case name"
					/>
					{#if r === 'running'}
						<Loader2 size={14} class="animate-spin text-secondary" />
					{:else if r}
						{#if r.passed}
							<span class="flex items-center gap-1 text-xs text-green-600">
								<Check size={14} /> Pass
							</span>
						{:else}
							<span class="flex items-center gap-1 text-xs text-red-600">
								<X size={14} /> Fail
							</span>
						{/if}
						{#if r.judge}
							<span class="text-2xs text-secondary">judge {r.judge.score}</span>
						{/if}
						<span class="text-2xs text-tertiary">{r.latency_ms}ms</span>
					{/if}
					<Button
						size="xs2"
						variant="default"
						startIcon={{ icon: Play }}
						disabled={r === 'running'}
						onclick={() => runCase(c)}
					>
						Run
					</Button>
					<Button
						size="xs2"
						variant="default"
						iconOnly
						startIcon={{ icon: Trash2 }}
						onclick={() => removeCase(c.id)}
					/>
				</div>

				<label class="flex flex-col gap-1 text-2xs text-secondary">
					Input message
					<textarea
						class="text-xs border border-border-light rounded px-2 py-1 bg-surface-input"
						rows="2"
						bind:value={c.input.user_message}
						placeholder="The user message to send to the agent"
					></textarea>
				</label>

				<label class="flex flex-col gap-1 text-2xs text-secondary">
					Judge checklist (one criterion per line)
					<textarea
						class="text-xs border border-border-light rounded px-2 py-1 bg-surface-input"
						rows="2"
						value={checklistText(c)}
						oninput={(e) => setChecklist(c, e.currentTarget.value)}
						placeholder="e.g. Mentions the order id&#10;Tone is professional"
					></textarea>
				</label>

				<div class="flex flex-col gap-1">
					<span class="text-2xs text-secondary">Assertions</span>
					{#each c.assertions ?? [] as a, i (i)}
						<div class="flex items-center gap-1 text-xs">
							<span class="text-2xs text-tertiary w-28 shrink-0">{a.kind}</span>
							{#if a.kind === 'contains' || a.kind === 'not_contains'}
								<input
									class="text-xs grow border border-border-light rounded px-2 py-0.5 bg-surface-input"
									bind:value={a.value}
									placeholder="substring"
								/>
							{:else if a.kind === 'regex'}
								<input
									class="text-xs grow border border-border-light rounded px-2 py-0.5 bg-surface-input"
									bind:value={a.pattern}
									placeholder="pattern"
								/>
							{:else if a.kind === 'json_path_equals'}
								<input
									class="text-xs w-32 border border-border-light rounded px-2 py-0.5 bg-surface-input"
									bind:value={a.path}
									placeholder="a.b.0"
								/>
								<input
									class="text-xs grow border border-border-light rounded px-2 py-0.5 bg-surface-input"
									value={typeof a.value === 'string' ? a.value : JSON.stringify(a.value)}
									oninput={(e) => {
										try {
											a.value = JSON.parse(e.currentTarget.value)
										} catch {
											a.value = e.currentTarget.value
										}
									}}
									placeholder="expected value"
								/>
							{:else}
								<span class="text-2xs text-tertiary grow">output is valid non-null JSON</span>
							{/if}
							<Button
								size="xs3"
								variant="default"
								iconOnly
								startIcon={{ icon: X }}
								onclick={() => removeAssertion(c, i)}
							/>
						</div>
					{/each}
					<div class="flex items-center gap-1">
						<div class="w-40">
							<Select
								items={ASSERTION_KINDS}
								bind:value={newAssertionKind[c.id]}
								placeholder="contains"
							/>
						</div>
						<Button
							size="xs3"
							variant="default"
							startIcon={{ icon: Plus }}
							onclick={() => addAssertion(c)}
						>
							Add assertion
						</Button>
					</div>
				</div>

				{#if r && r !== 'running'}
					<div class="border-t border-border-light pt-2">
						<button
							class="flex items-center gap-1 text-2xs text-secondary"
							onclick={() => (expanded[c.id] = !expanded[c.id])}
						>
							{#if expanded[c.id]}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
							Result
						</button>
						{#if expanded[c.id]}
							<div class="mt-1 flex flex-col gap-1 text-2xs">
								{#if r.error}
									<div class="text-red-600">{r.error}</div>
								{/if}
								{#if r.judge}
									<div><span class="text-tertiary">Judge:</span> {r.judge.summary}</div>
								{/if}
								{#each r.assertions ?? [] as ar, ai (ai)}
									<div class={ar.passed ? 'text-green-600' : 'text-red-600'}>
										{ar.passed ? '✓' : '✗'} {ar.assertion.kind}
										{#if ar.detail}<span class="text-tertiary"> — {ar.detail}</span>{/if}
									</div>
								{/each}
								{#if r.output !== undefined}
									<pre class="bg-surface-secondary rounded p-2 overflow-auto max-h-48 whitespace-pre-wrap">{outputText(r.output)}</pre>
								{/if}
							</div>
						{/if}
					</div>
				{/if}
			</div>
		{/each}
	{/if}
</div>
