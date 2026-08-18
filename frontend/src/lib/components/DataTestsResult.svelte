<script lang="ts">
	// Renders the per-test breakdown a managed `// materialize` run attaches to
	// its result as `data_tests: [{ test, violating, sample? }]`. Shown above
	// the raw result so "which tests ran, and which passed/failed" is clear at
	// a glance. A failed test may carry `sample` — a bounded, unordered sample
	// of its violating rows — rendered as an expandable table. The sample is
	// optional by contract: its absence only means no rows to show.
	import { CheckCircle2, ChevronDown, ChevronRight, FlaskConical, XCircle } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import AutoDataTable from '$lib/components/table/AutoDataTable.svelte'
	import { SvelteSet } from 'svelte/reactivity'

	let {
		tests
	}: { tests: Array<{ test: string; violating: number; sample?: Record<string, any>[] }> } =
		$props()

	let failed = $derived(tests.filter((t) => t.violating > 0).length)
	let expanded = new SvelteSet<string>()
</script>

<div
	class="mb-2 rounded-md border bg-surface-secondary text-xs overflow-hidden {failed > 0
		? 'border-red-300 dark:border-red-900/60'
		: 'border-border'}"
>
	<div class="flex items-center gap-1.5 px-2 py-1 border-b border-border text-secondary">
		<FlaskConical size={13} />
		<span class="font-medium">
			{tests.length} data test{tests.length === 1 ? '' : 's'}
		</span>
		<span
			class={failed > 0
				? 'text-red-600 dark:text-red-400'
				: 'text-emerald-600 dark:text-emerald-400'}
		>
			· {failed > 0 ? `${failed} failed` : 'all passed'}
		</span>
	</div>
	<ul class="divide-y divide-border">
		{#each tests as t (t.test)}
			<li class="px-2 py-1">
				<div class="flex items-center gap-1.5 font-mono">
					{#if t.violating > 0}
						<XCircle size={13} class="shrink-0 text-red-600 dark:text-red-400" />
						<span class="sr-only">failed:</span>
						<span class="text-primary">{t.test}</span>
						<span class="text-red-600 dark:text-red-400"
							>— {t.violating} violating row{t.violating === 1 ? '' : 's'}</span
						>
						{#if t.sample && t.sample.length > 0}
							<Button
								variant="subtle"
								unifiedSize="sm"
								startIcon={{ icon: expanded.has(t.test) ? ChevronDown : ChevronRight }}
								onclick={() => {
									if (!expanded.delete(t.test)) expanded.add(t.test)
								}}
							>
								{expanded.has(t.test) ? 'hide sample' : `view sample (${t.sample.length})`}
							</Button>
						{/if}
					{:else}
						<CheckCircle2 size={13} class="shrink-0 text-emerald-600 dark:text-emerald-400" />
						<span class="sr-only">passed:</span>
						<span class="text-secondary">{t.test}</span>
					{/if}
				</div>
				{#if t.violating > 0 && t.sample && t.sample.length > 0 && expanded.has(t.test)}
					<div class="mt-1 mb-1">
						<div class="text-2xs text-tertiary mb-1">
							sample of the violating rows ({t.sample.length} of {t.violating}, unordered)
						</div>
						<AutoDataTable objects={t.sample} />
					</div>
				{/if}
			</li>
		{/each}
	</ul>
</div>
