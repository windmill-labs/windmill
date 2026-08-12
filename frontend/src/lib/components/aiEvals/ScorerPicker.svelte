<script lang="ts">
	import { Button } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Label from '$lib/components/Label.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { Plus, X } from 'lucide-svelte'
	import { ResourceService } from '$lib/gen'

	type Scorer = { kind: 'script' | 'flow' | 'agent'; path: string; name?: string }

	let {
		scorers = $bindable(),
		workspace = undefined
	}: {
		scorers: Scorer[]
		workspace?: string
	} = $props()

	let kind = $state<'script' | 'flow' | 'agent'>('script')
	let path = $state('')
	let agentOptions = $state<{ label: string; value: string }[]>([])

	// A judge is an ai_agent resource, so offer them by name rather than making the user recall a
	// path; scripts and flows are typed in, as elsewhere in the editor.
	$effect(() => {
		if (kind !== 'agent' || !workspace) return
		ResourceService.listResource({ workspace, resourceType: 'ai_agent', perPage: 1000 })
			.then((rs) => (agentOptions = rs.map((r) => ({ label: r.path, value: r.path }))))
			.catch(() => (agentOptions = []))
	})

	function add() {
		if (!path) return
		scorers = [...scorers, { kind, path }]
		path = ''
	}
</script>

<Label
	label="Scorers"
	tooltip="A scorer is any runnable taking the case input, the agent's answer and the expected value, and returning a number (or a boolean, or an object with a score). An LLM-as-judge is just a reusable agent used here."
>
	{#if scorers.length > 0}
		<div class="flex flex-wrap gap-1">
			{#each scorers as scorer, index (scorer.path + index)}
				<Badge color="gray">
					<span class="text-tertiary">{scorer.kind}</span>
					<span>{scorer.path}</span>
					<button
						class="rounded-full p-0.5 text-tertiary hover:bg-surface-hover hover:text-primary"
						title="Remove scorer"
						aria-label="Remove scorer"
						onclick={() => (scorers = scorers.filter((_, i) => i !== index))}
					>
						<X size={11} />
					</button>
				</Badge>
			{/each}
		</div>
	{/if}

	<div class="flex items-center gap-1">
		<div class="w-24">
			<Select
				items={[
					{ label: 'script', value: 'script' },
					{ label: 'flow', value: 'flow' },
					{ label: 'agent', value: 'agent' }
				]}
				bind:value={kind}
				class="text-xs"
			/>
		</div>
		<div class="grow min-w-0">
			{#if kind === 'agent'}
				<Select items={agentOptions} bind:value={path} placeholder="Judge agent" class="text-xs" />
			{:else}
				<TextInput
					bind:value={path}
					size="sm"
					inputProps={{
						placeholder: kind === 'script' ? 'f/folder/script' : 'f/folder/flow',
						// A path typed but never added is a scorer the run silently would not have.
						onkeydown: (e: KeyboardEvent) => e.key === 'Enter' && add()
					}}
				/>
			{/if}
		</div>
		<Button
			variant="default"
			size="xs2"
			startIcon={{ icon: Plus }}
			iconOnly
			title="Add scorer"
			disabled={!path}
			onclick={add}
		/>
	</div>
</Label>
