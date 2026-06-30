<script lang="ts">
	import { Sparkles } from 'lucide-svelte'
	import DrillPicker from '$lib/components/DrillPicker.svelte'
	import type { DrillLeaf, DrillNode } from '$lib/components/drillPicker'
	import type { AiSkillListItem } from './global/core'

	interface Props {
		skills: AiSkillListItem[]
		onSelect: (skill: AiSkillListItem) => void
		setShowing?: (showing: boolean) => void
		externalFilter?: string
		autoFocus?: boolean
	}

	let { skills, onSelect, setShowing, externalFilter, autoFocus = true }: Props = $props()

	type DrillPickerHandle = {
		handleKeydown: (e: KeyboardEvent) => void
	}

	let inner = $state<DrillPickerHandle | undefined>(undefined)

	const tree = $derived<DrillNode<AiSkillListItem>[]>(
		skills.map((skill) => ({
			type: 'leaf' as const,
			key: `skill:${skill.name}`,
			label: `/${skill.name}`,
			secondary: skill.description,
			searchableText: `${skill.name} ${skill.description}`,
			data: skill
		}))
	)

	export function handleKeydown(e: KeyboardEvent) {
		inner?.handleKeydown(e)
	}

	function handlePick(leaf: DrillLeaf<AiSkillListItem>) {
		onSelect(leaf.data)
	}

	function onDocumentKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && !e.defaultPrevented) {
			setShowing?.(false)
		}
	}

	$effect(() => {
		document.addEventListener('keydown', onDocumentKeydown)
		return () => document.removeEventListener('keydown', onDocumentKeydown)
	})
</script>

{#snippet skillIcon(_leaf: DrillLeaf<AiSkillListItem>)}
	<Sparkles size={12} class="shrink-0 text-tertiary" />
{/snippet}

<div class="w-[min(340px,calc(100vw-20px))] max-h-64 overflow-hidden">
	<DrillPicker
		bind:this={inner}
		{tree}
		onPick={handlePick}
		{externalFilter}
		{autoFocus}
		leafIcon={skillIcon}
		flush
	/>
</div>
