<script lang="ts">
	import { sendUserToast } from '$lib/toast'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import type { SideEffectAction } from './types'

	const { componentControl, runnableComponents } = getContext<AppViewerContext>('AppViewerContext')

	export async function handleSideEffect(
		sideEffect: SideEffectAction,
		success: boolean,
		recomputeIds?: string[],
		errorMessage?: string
	) {
		if (recomputeIds && success) {
			recomputeIds.forEach((id) => $runnableComponents?.[id]?.cb?.map((cb) => cb?.()))
		}
		if (!sideEffect) return

		if (sideEffect.selected == 'none') return

		switch (sideEffect.selected) {
			case 'setTab':
				let setTab = sideEffect?.configuration.setTab?.setTab
				if (!setTab) return
				if (typeof setTab === 'function') {
					setTab = await setTab()
				}
				if (Array.isArray(setTab)) {
					setTab.forEach((tab) => {
						if (tab) {
							const { id, index } = tab
							$componentControl[id].setTab?.(index)
						}
					})
				}
				break
			case 'gotoUrl':
				let gotoUrl = sideEffect?.configuration?.gotoUrl?.url

				if (!gotoUrl) return
				if (typeof gotoUrl === 'function') {
					gotoUrl = await gotoUrl()
				}
				const newTab = sideEffect?.configuration?.gotoUrl?.newTab

				if (newTab) {
					window.open(gotoUrl, '_blank')
				} else {
					window.location.href = gotoUrl
				}

				break
			case 'sendToast': {
				let message = sideEffect?.configuration?.sendToast?.message

				if (!message) return
				if (typeof message === 'function') {
					message = await message()
				}
				sendUserToast(message, !success)
				break
			}
			case 'sendErrorToast': {
				let message = sideEffect?.configuration?.sendErrorToast?.message
				const appendError = sideEffect?.configuration?.sendErrorToast?.appendError

				if (!message) return

				if (typeof message === 'function') {
					message = await message()
				}

				sendUserToast(message, true, [], appendError ? errorMessage : undefined)
				break
			}
			case 'openModal': {
				const modalId = sideEffect?.configuration?.openModal?.modalId
				if (modalId) {
					$componentControl[modalId].openModal?.()
				}
				break
			}
			case 'closeModal': {
				const modalId = sideEffect?.configuration?.closeModal?.modalId

				if (!modalId) return

				$componentControl[modalId].closeModal?.()
				break
			}
			case 'open': {
				const id = sideEffect?.configuration?.open?.id

				if (!id) return

				$componentControl[id].open?.()
				break
			}
			case 'close': {
				const id = sideEffect?.configuration?.close?.id

				if (!id) return

				$componentControl[id].close?.()
				break
			}
			case 'clearFiles': {
				const id = sideEffect?.configuration?.clearFiles?.id

				if (!id) return

				$componentControl[id].clearFiles?.()
				break
			}
			default:
				break
		}
	}
</script>
