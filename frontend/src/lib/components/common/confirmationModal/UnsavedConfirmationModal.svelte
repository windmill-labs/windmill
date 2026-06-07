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
	}

	let {
		getInitialAndModifiedValues = undefined,
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

			const state = getInitialAndModifiedValues?.()
			savedValue = state?.savedValue
			modifiedValue = state?.modifiedValue

			async function openModal() {
				newNavigationState.cancel()
				open = true
			}
			if (savedValue && modifiedValue) {
				const draftOrDeployed = cleanValueProperties((savedValue.draft || savedValue) ?? {})
				const current = cleanValueProperties(modifiedValue ?? {})

				if (
					orderedJsonStringify(replaceFalseWithUndefined(draftOrDeployed)) ===
					orderedJsonStringify(replaceFalseWithUndefined(current))
				) {
					if (!tabMode) {
						bypassBeforeNavigate = true
					}
					additionalExitAction?.()
				} else {
					await openModal()
				}
			} else {
				await openModal()
			}
		} else if (bypassBeforeNavigate) {
			bypassBeforeNavigate = false
		}
	})
</script>

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
	</div>
</ConfirmationModal>
