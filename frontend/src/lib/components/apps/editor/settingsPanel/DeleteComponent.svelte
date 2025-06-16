<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '../../types'
	import { deleteGridItem, findComponentSettings } from '../appUtils'
	import { push } from '$lib/history'

	export let onDelete: (() => void) | undefined = undefined
	export let noGrid = false

	const {
		app,
		runnableComponents,
		selectedComponent,
		worldStore,
		focusedGrid,
		errorByComponent,
		componentControl
	} = getContext<AppViewerContext>('AppViewerContext')

	const { history, movingcomponents } = getContext<AppEditorContext>('AppEditorContext')

	export function removeGridElement() {
		const id = $selectedComponent?.[0]
		const componentSetting = findComponentSettings(app.val, id)
		push(history, app.val)

		const onDeleteComponentControl = id ? $componentControl[id]?.onDelete : undefined
		if (onDeleteComponentControl) {
			onDeleteComponentControl()
		}
		if (onDelete) {
			onDelete()
		}

		if (id) {
			delete $worldStore.outputsById[id]
			delete $errorByComponent[id]

			if ($movingcomponents?.includes(id)) {
				$movingcomponents = $movingcomponents.filter((_id) => _id !== id)
			}
		}

		$selectedComponent = undefined
		$focusedGrid = undefined
		if (componentSetting?.item && !noGrid) {
			let ids = deleteGridItem(app.val, componentSetting?.item.data, componentSetting?.parent)
			for (const key of ids) {
				delete $runnableComponents[key]
			}
		}

		if (componentSetting?.item?.data?.id) {
			delete $runnableComponents[componentSetting?.item?.data?.id]
		}
		app.val = app.val
		$runnableComponents = $runnableComponents

		onDelete?.()
	}
</script>
