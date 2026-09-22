<script lang="ts">
	import { Button } from '$lib/components/common'
	import DiffEditor from '$lib/components/DiffEditor.svelte'
	import ToolCodeDiffView from '$lib/components/copilot/chat/ToolCodeDiffView.svelte'
	import type { ToolCodeDiff } from '$lib/components/copilot/chat/shared'

	type ComparisonCase = {
		id: string
		label: string
		diff: ToolCodeDiff
	}

	const stableLines = Array.from(
		{ length: 14 },
		(_, index) => `	const stableValue${index + 1} = getStableValue(${index + 1})`
	)

	const cases: ComparisonCase[] = [
		{
			id: 'typescript-replacement',
			label: 'TypeScript replacement',
			diff: {
				lang: 'typescript',
				before: `export function greeting(name: string) {
	return \`Hello, \${name}\`
}`,
				after: `export function greeting(name: string) {
	const message = \`Hello, \${name}!\`
	return message
}`
			}
		},
		{
			id: 'long-unchanged-sections',
			label: 'Long unchanged sections',
			diff: {
				lang: 'typescript',
				before: [
					'export function buildSummary() {',
					...stableLines,
					'\treturn formatSummary("draft")',
					'}'
				].join('\n'),
				after: [
					'export function buildSummary() {',
					...stableLines,
					'\treturn formatSummary("published")',
					'}'
				].join('\n')
			}
		},
		{
			id: 'json-structure',
			label: 'JSON structure',
			diff: {
				lang: 'json',
				before: `{
	"name": "daily-report",
	"schedule": "0 9 * * 1-5",
	"enabled": false
}`,
				after: `{
	"name": "daily-report",
	"schedule": "0 9 * * 1-5",
	"timezone": "Europe/Paris",
	"enabled": true
}`
			}
		},
		{
			id: 'python-add-remove',
			label: 'Python additions and removals',
			diff: {
				lang: 'python',
				before: `def notify(users):
	for user in users:
		send_email(user, "Weekly update")

	return len(users)`,
				after: `def notify(users):
	active_users = [user for user in users if user.active]
	for user in active_users:
		send_email(user, "Your weekly update")

	return len(active_users)`
			}
		}
	]

	let activeCaseId = $state(cases[0].id)
	let activeCase = $derived(cases.find((item) => item.id === activeCaseId) ?? cases[0])
</script>

<svelte:head>
	<title>Diff comparison</title>
</svelte:head>

<main class="mx-auto max-w-[1440px] p-6">
	<div class="mb-5 flex flex-wrap gap-2">
		{#each cases as item}
			<Button
				unifiedSize="xs"
				variant="subtle"
				selected={activeCaseId === item.id}
				onclick={() => (activeCaseId = item.id)}>{item.label}</Button
			>
		{/each}
	</div>

	<div class="grid gap-5 lg:grid-cols-2">
		<section class="min-w-0">
			<h1 class="mb-2 text-sm font-semibold">Monaco</h1>
			<div class="h-[400px] overflow-hidden border border-slate-200 bg-white">
				{#key activeCase.id}
					<DiffEditor
						className="h-full"
						defaultLang={activeCase.diff.lang}
						defaultModifiedLang={activeCase.diff.lang}
						defaultOriginal={activeCase.diff.before}
						defaultModified={activeCase.diff.after}
						inlineDiff
						open
						readOnly
					/>
				{/key}
			</div>
		</section>

		<section class="min-w-0">
			<h2 class="mb-2 text-sm font-semibold">Native</h2>
			<div class="h-[400px] overflow-auto border border-slate-200 bg-white">
				<ToolCodeDiffView diff={activeCase.diff} />
			</div>
		</section>
	</div>
</main>
