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
	import { page } from '$app/stores'
	import type { GetInitialAndModifiedValues } from './unsavedTypes'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'

	export let getInitialAndModifiedValues: GetInitialAndModifiedValues = undefined
	export let diffDrawer: DiffDrawer | undefined = undefined
	export let additionalExitAction: () => void = () => {}
	let savedValue: Value | undefined = undefined
	let modifiedValue: Value | undefined = undefined

	let bypassBeforeNavigate = false
	let open = false
	let goingTo: URL | undefined = undefined

	beforeNavigate(async (newNavigationState) => {
		if (
			!bypassBeforeNavigate &&
			getInitialAndModifiedValues &&
			newNavigationState.to &&
			newNavigationState.to.url != $page.url &&
			newNavigationState.to.url.pathname !== newNavigationState.from?.url.pathname
		) {
			// console.log('going to', newNavigationState.to.url)
			goingTo = newNavigationState.to.url

			const state = getInitialAndModifiedValues?.()
			savedValue = state?.savedValue
			modifiedValue = state?.modifiedValue

			async function openModal() {
				newNavigationState.cancel()
				open = true
			}
			if (savedValue && modifiedValue) {
				const draftOrDeployed = cleanValueProperties({
					...((savedValue.draft || savedValue) ?? {}),
					path: undefined
				})
				const current = cleanValueProperties({ ...(modifiedValue ?? {}), path: undefined })

				if (
					orderedJsonStringify(replaceFalseWithUndefined(draftOrDeployed)) ===
					orderedJsonStringify(replaceFalseWithUndefined(current))
				) {
					bypassBeforeNavigate = true
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
				color="light"
				variant="border"
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
