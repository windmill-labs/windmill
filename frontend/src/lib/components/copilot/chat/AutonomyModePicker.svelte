<script lang="ts">
	import { ArrowRight, ChevronDown } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import type { Item } from '$lib/utils'
	import type { AIAutonomyMode } from './AIChatManager.svelte'
	import { AIWorkMode, type WorkModeLock } from './workMode'
	import {
		autonomyModeOption,
		availableAutonomyModeOptions,
		resolveAutonomyMode,
		workModeOption,
		workModeOptions,
		type AutonomyAvailability
	} from './autonomyModes'

	let {
		mode,
		availability,
		onChange,
		work = undefined
	}: {
		mode: AIAutonomyMode
		availability: AutonomyAvailability
		onChange: (mode: AIAutonomyMode) => void
		/** A session chat's Use / Plan / Build, shown as the picker's first section. */
		work?: {
			mode: AIWorkMode
			planAvailable: boolean
			/** Holds the chat in Use, and says why. */
			lock?: WorkModeLock
			onChange: (mode: AIWorkMode) => void
		}
	} = $props()

	const effective = $derived(resolveAutonomyMode(mode, availability))
	const option = $derived(autonomyModeOption(effective))
	// Plan where the chat no longer offers it falls back to Build, as the manager does.
	const workOption = $derived(
		!work
			? undefined
			: workModeOption(
					work.lock
						? AIWorkMode.USE
						: work.mode === AIWorkMode.PLAN && !work.planAvailable
							? AIWorkMode.BUILD
							: work.mode
				)
	)
	const postureLabel = $derived(option.shortLabel ?? option.label)

	function items(): Item[] {
		const sections = !!work
		const postures: Item[] = availableAutonomyModeOptions(availability).map((o, i) => ({
			displayName: o.label,
			sectionTitle: sections && i === 0 ? 'Confirmations' : undefined,
			separatorTop: sections && i === 0,
			selected: effective === o.mode,
			action: () => onChange(o.mode)
		}))
		const w = work
		if (!w) return postures
		const lockedOut = (m: AIWorkMode) => !!w.lock && m !== AIWorkMode.USE
		const workItems: Item[] = workModeOptions
			.filter((o) => o.mode !== AIWorkMode.PLAN || w.planAvailable || w.lock)
			.map((o, i) => ({
				displayName: o.label,
				description: o.description,
				icon: o.icon,
				// `iconColor` takes a color value; these are text classes.
				iconProps: { class: `shrink-0 ${o.iconColor}` },
				sectionTitle: i === 0 ? 'What the chat can do' : undefined,
				selected: workOption?.mode === o.mode,
				disabled: lockedOut(o.mode),
				tooltip: lockedOut(o.mode) ? w.lock?.reason : undefined,
				action: () => w.onChange(o.mode)
			}))
		const escape = w.lock?.escape
		return [
			...workItems,
			...(escape ? [{ displayName: escape.label, icon: ArrowRight, action: escape.action }] : []),
			...postures
		]
	}
</script>

<DropdownV2 {items} placement="bottom-start" fixedHeight={false} customWidth={work ? 260 : 240}>
	{#snippet buttonReplacement()}
		<Button
			nonCaptureEvent
			unifiedSize="2xs"
			variant="subtle"
			title={workOption
				? (work?.lock?.reason ?? `${workOption.description} ${option.tooltip(availability)}`)
				: option.tooltip(availability)}
			btnClasses={workOption?.triggerClass ?? ''}
			startIcon={{
				icon: (workOption ?? option).icon,
				classes: (workOption ?? option).iconColor
			}}
			endIcon={{ icon: ChevronDown }}
		>
			{workOption ? `${workOption.label} · ${postureLabel}` : postureLabel}
		</Button>
	{/snippet}
</DropdownV2>
