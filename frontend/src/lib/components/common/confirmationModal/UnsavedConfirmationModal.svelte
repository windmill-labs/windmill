<script lang="ts">
	import ConfirmationModal from './ConfirmationModal.svelte'
	import { beforeNavigate } from '$app/navigation'
	import { goto as gotoUrl } from '$app/navigation'
	import {
		cleanValueProperties,
		orderedJsonStringify,
		replaceFalseWithUndefined,
		type Value
	} from '$lib/utils'
	import { page } from '$app/state'
	import type { GetInitialAndModifiedValues } from './unsavedTypes'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'

	interface Props {
		getInitialAndModifiedValues?: GetInitialAndModifiedValues
		additionalExitAction?: () => void
		triggerOnSearchParamsChange?: boolean
		onDiscardChanges?: () => void
		tabMode?: boolean
		/** Alternative dirty check. When provided it REPLACES the value diff:
		 *  the modal engages whenever it returns true (the value-diff props can
		 *  be omitted). The full-page editors pass the auto-save-off "parked
		 *  unsaved changes" signal here. */
		hasUnsavedChanges?: () => boolean
		/** Adds a line to the confirmation telling the user they can enable
		 *  auto-save to persist a draft automatically. */
		showAutosaveTips?: boolean
	}

	let {
		getInitialAndModifiedValues = undefined,
		additionalExitAction = () => {},
		triggerOnSearchParamsChange = false,
		onDiscardChanges = undefined,
		tabMode = false,
		hasUnsavedChanges = undefined,
		showAutosaveTips = false
	}: Props = $props()
	let savedValue: Value | undefined = $state(undefined)
	let modifiedValue: Value | undefined = $state(undefined)

	let bypassBeforeNavigate = $state(false)
	let open = $state(false)
	let goingTo: URL | undefined = $state(undefined)

	// The modal is wired up when either dirty-detection mode is configured.
	let dirtyDetectionActive = $derived(!!getInitialAndModifiedValues || !!hasUnsavedChanges)

	// Mirrors the modal condition: dirty when values differ, or when either
	// value is missing (e.g. a never-saved draft). Also refreshes
	// savedValue/modifiedValue for the diff drawer. `hasUnsavedChanges`, when
	// passed, short-circuits the value diff with the caller's own predicate.
	function checkUnsavedChanges(): boolean {
		if (hasUnsavedChanges) return hasUnsavedChanges()
		const state = getInitialAndModifiedValues?.()
		savedValue = state?.savedValue
		modifiedValue = state?.modifiedValue

		if (savedValue && modifiedValue) {
			const draftOrDeployed = cleanValueProperties((savedValue.draft || savedValue) ?? {})
			const current = cleanValueProperties(modifiedValue ?? {})

			return (
				orderedJsonStringify(replaceFalseWithUndefined(draftOrDeployed)) !==
				orderedJsonStringify(replaceFalseWithUndefined(current))
			)
		}
		return true
	}

	beforeNavigate(async (newNavigationState) => {
		if (
			!bypassBeforeNavigate &&
			dirtyDetectionActive &&
			newNavigationState.to &&
			((newNavigationState.to.url != page.url &&
				newNavigationState.to.url.pathname !== newNavigationState.from?.url.pathname) ||
				(triggerOnSearchParamsChange && newNavigationState.to.url.search != page.url.search))
		) {
			goingTo = newNavigationState.to.url

			if (checkUnsavedChanges()) {
				newNavigationState.cancel()
				open = true
			} else {
				if (!tabMode) {
					bypassBeforeNavigate = true
				}
				additionalExitAction?.()
			}
		} else if (bypassBeforeNavigate) {
			bypassBeforeNavigate = false
		}
	})

	function onBeforeUnload(event: BeforeUnloadEvent) {
		if (!bypassBeforeNavigate && dirtyDetectionActive && checkUnsavedChanges()) {
			// Triggers the browser's native "leave site?" confirmation
			event.preventDefault()
			// Required by some browsers (legacy mechanism)
			event.returnValue = true
		}
	}
</script>

<svelte:window onbeforeunload={onBeforeUnload} />

{#if open}
	<div
		style="display: none"
		use:triggerableByAI={{
			id: 'unsaved-changes-confirmation-modal',
			description: 'Unsaved changes confirmation modal. Needs user confirmation to leave the page.'
		}}
	></div>
{/if}

<ConfirmationModal
	{open}
	title="Unsaved changes detected"
	confirmationText="Discard changes"
	on:canceled={() => {
		open = false
	}}
	on:confirmed={() => {
		open = false
		// Discard changes before navigating
		onDiscardChanges?.()
		if (goingTo) {
			bypassBeforeNavigate = true
			additionalExitAction?.()
			gotoUrl(goingTo)
		}
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to discard the changes you have made? </span>
		{#if showAutosaveTips}
			<span class="text-xs text-tertiary">
				Auto-save is off, so these changes are not saved as a draft. Enable auto-save (the cloud
				icon in the editor toolbar) to persist your changes automatically, or press Ctrl/Cmd+S to
				save the current draft before leaving.
			</span>
		{/if}
	</div>
</ConfirmationModal>
