<script lang="ts">
	// Renders the per-test breakdown a managed `// materialize` run attaches to
	// its result as `data_tests: [{ test, violating }]`. Shown above the raw
	// result so "which tests ran, and which passed/failed" is clear at a glance.
	// (On a failed run the job result is the error message — which already lists
	// every test — so this success-path checklist and that error text together
	// cover both outcomes.)
	import { CheckCircle2, FlaskConical, XCircle } from 'lucide-svelte'

	let { tests }: { tests: Array<{ test: string; violating: number }> } = $props()

	let failed = $derived(tests.filter((t) => t.violating > 0).length)
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
			<li class="flex items-center gap-1.5 px-2 py-1 font-mono">
				{#if t.violating > 0}
					<XCircle size={13} class="shrink-0 text-red-600 dark:text-red-400" />
					<span class="sr-only">failed:</span>
					<span class="text-primary">{t.test}</span>
					<span class="text-red-600 dark:text-red-400"
						>— {t.violating} violating row{t.violating === 1 ? '' : 's'}</span
					>
				{:else}
					<CheckCircle2 size={13} class="shrink-0 text-emerald-600 dark:text-emerald-400" />
					<span class="sr-only">passed:</span>
					<span class="text-secondary">{t.test}</span>
				{/if}
			</li>
		{/each}
	</ul>
</div>
