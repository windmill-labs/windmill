<script lang="ts">
	import DrillPicker from '$lib/components/DrillPicker.svelte'
	import type { DrillLeaf, DrillNode } from '$lib/components/drillPicker'
	import type { ChatCommandItem } from './global/core'
	import { ambiguousSkillNames } from './skills/skillResources'

	interface Props {
		skills: ChatCommandItem[]
		onSelect: (skill: ChatCommandItem) => void
		setShowing?: (showing: boolean) => void
		externalFilter?: string
		autoFocus?: boolean
	}

	let { skills, onSelect, setShowing, externalFilter, autoFocus = true }: Props = $props()

	type DrillPickerHandle = {
		handleKeydown: (e: KeyboardEvent) => void
	}

	let inner = $state<DrillPickerHandle | undefined>(undefined)

	const SECTION_LABELS: Record<NonNullable<ChatCommandItem['kind']>, string> = {
		action: 'Actions',
		skill: 'Skills'
	}

	// Two folders can each hold a skill of the same name, and `/name` cannot then
	// say which one is meant. Those rows show their path so the two are at least
	// distinguishable; unambiguous rows stay bare, with the description in the
	// hover tooltip (rowTooltip below) and in `searchableText` so filtering by it
	// keeps working.
	const ambiguous = $derived(ambiguousSkillNames(skills.filter((s) => s.path !== undefined)))

	const tree = $derived<DrillNode<ChatCommandItem>[]>(
		skills.map((skill) => ({
			type: 'leaf' as const,
			// Keyed by path where there is one: names are not unique across folders,
			// and a duplicate key breaks the keyed list and its ambiguous-resolve nav.
			key: `skill:${skill.path ?? skill.name}`,
			label: `/${skill.name}`,
			secondary: ambiguous.has(skill.name) ? skill.path : undefined,
			searchableText: `${skill.name} ${skill.path ?? ''} ${skill.description}`,
			section: skill.kind ? SECTION_LABELS[skill.kind] : undefined,
			data: skill
		}))
	)

	export function handleKeydown(e: KeyboardEvent) {
		inner?.handleKeydown(e)
	}

	function handlePick(leaf: DrillLeaf<ChatCommandItem>) {
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

<!-- This wrapper is the scroll container: the flush DrillPicker sizes to its
     content (h-full can't resolve against a max-h-only parent), so overflow
     must scroll here or the list is just clipped at 16rem. -->
<div class="w-[min(340px,calc(100vw-20px))] max-h-64 overflow-y-auto">
	<DrillPicker
		bind:this={inner}
		{tree}
		onPick={handlePick}
		{externalFilter}
		{autoFocus}
		rowTooltip={(leaf) => leaf.data.description}
		flush
	/>
</div>
