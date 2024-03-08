<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext, GridItem } from '../../types'
	import { deleteGridItem } from '../appUtils'
	import { push } from '$lib/history'

	export let componentSettings:
		| {
				item: GridItem
				parent: string
		  }[]
		| any = []
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
		push(history, $app)

		componentSettings?.forEach((componentSetting) => {
			const id = componentSetting?.item?.id
			const onDeleteComponentControl = id ? $componentControl[id]?.onDelete : undefined
			if (onDeleteComponentControl) {
				onDeleteComponentControl()
			}
			if (onDelete) {
				onDelete()
			}

			let cId = componentSetting?.item.id
			if (cId) {
				delete $worldStore.outputsById[cId]
				delete $errorByComponent[cId]

				if ($movingcomponents?.includes(cId)) {
					$movingcomponents = $movingcomponents.filter((id) => id !== cId)
				}
			}

			$selectedComponent = undefined
			$focusedGrid = undefined
			if (componentSetting?.item && !noGrid) {
				let ids = deleteGridItem($app, componentSetting?.item.data, componentSetting?.parent)
				for (const key of ids) {
					delete $runnableComponents[key]
				}
			}

			if (componentSetting?.item?.data?.id) {
				delete $runnableComponents[componentSetting?.item?.data?.id]
			}
			$app = $app
			$runnableComponents = $runnableComponents

			onDelete?.()
		})
	}
</script>
