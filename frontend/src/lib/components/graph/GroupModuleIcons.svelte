<script lang="ts">
	import FlowModuleIcon from '$lib/components/flows/FlowModuleIcon.svelte'
	import type { FlowModule } from '$lib/gen'

	interface Props {
		modules: FlowModule[]
	}

	let { modules }: Props = $props()

	function iconKey(mod: FlowModule): string {
		const type = mod.value?.type
		if (type === 'aiagent') return 'aiagent'
		if (type === 'rawscript') return `lang:${mod.value.language}`
		if (mod.summary === 'Terminate flow') return 'terminate'
		if (type === 'identity') return 'identity'
		if (type === 'flow') return 'flow'
		if (type === 'forloopflow' || type === 'whileloopflow') return 'loop'
		if (type === 'branchone' || type === 'branchall') return 'branch'
		if (type === 'script') {
			const path = mod.value?.path
			if (path?.startsWith('hub/')) return `hub:${path.split('/')[2]}`
			return 'script'
		}
		return 'unknown'
	}

	let displayModules = $derived.by(() => {
		const seen = new Set<string>()
		const unique: FlowModule[] = []
		for (const mod of modules) {
			const key = iconKey(mod)
			if (!seen.has(key)) {
				seen.add(key)
				unique.push(mod)
				if (unique.length >= 5) break
			}
		}
		return unique
	})
</script>

<div class="flex items-center gap-1">
	{#each displayModules as mod, i (mod.id)}
		<div
			class="w-6 h-6 rounded-full overflow-hidden bg-surface-tertiary flex items-center justify-center shrink-0 shadow-sm"
		>
			<FlowModuleIcon module={mod} size={14} />
		</div>
	{/each}
</div>
