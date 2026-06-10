<script lang="ts">
	import ConfirmationModal from './ConfirmationModal.svelte'
	import { beforeNavigate } from '$app/navigation'
	import { goto as gotoUrl } from '$app/navigation'
	import Button from '../button/Button.svelte'
	import type DiffDrawer from '$lib/components/DiffDrawer.svelte'
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
		diffDrawer?: DiffDrawer | undefined
		additionalExitAction?: () => void
		triggerOnSearchParamsChange?: boolean
		onDiscardChanges?: () => void
		tabMode?: boolean
	}

	let {
		getInitialAndModifiedValues = undefined,
		diffDrawer = undefined,
		additionalExitAction = () => {},
		triggerOnSearchParamsChange = false,
		onDiscardChanges = undefined,
		tabMode = false
	}: Props = $props()
	let savedValue: Value | undefined = $state(undefined)
	let modifiedValue: Value | undefined = $state(undefined)

	let bypassBeforeNavigate = $state(false)
	let open = $state(false)
	let goingTo: URL | undefined = $state(undefined)

	// Mirrors the modal condition: dirty when values differ, or when either
	// value is missing (e.g. a never-saved draft). Also refreshes
	// savedValue/modifiedValue for the diff drawer.
	function hasUnsavedChanges(): boolean {
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
			getInitialAndModifiedValues &&
			newNavigationState.to &&
			((newNavigationState.to.url != page.url &&
				newNavigationState.to.url.pathname !== newNavigationState.from?.url.pathname) ||
				(triggerOnSearchParamsChange && newNavigationState.to.url.search != page.url.search))
		) {
			goingTo = newNavigationState.to.url

			if (hasUnsavedChanges()) {
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
		if (!bypassBeforeNavigate && getInitialAndModifiedValues && hasUnsavedChanges()) {
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
		{#if savedValue && modifiedValue && diffDrawer}
			<Button
				wrapperClasses="self-start"
				variant="default"
				size="xs"
				on:click={() => {
					if (!savedValue || !modifiedValue) {
						return
					}
					open = false
					diffDrawer?.openDrawer()
					diffDrawer?.setDiff({
						mode: 'normal',
						deployed: savedValue,
						draft: savedValue.draft,
						current: modifiedValue,
						defaultDiffType: 'draft',
						button: {
							text: 'Leave anyway',
							onClick: () => {
								if (goingTo) {
									bypassBeforeNavigate = true
									additionalExitAction?.()
									gotoUrl(goingTo)
								}
							}
						}
					})
				}}
				>Show diff
			</Button>
		{/if}
	</div>
</ConfirmationModal>
