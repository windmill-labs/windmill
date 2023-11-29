<script lang="ts">
	import { Badge } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { getNextId } from '$lib/components/flows/idUtils'
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { AppViewerContext, BaseAppComponent } from '../../types'
	import { appComponentFromType } from '../appUtils'
	import type { ButtonComponent } from '../component'
	import PanelSection from './common/PanelSection.svelte'
	import { Plus, Trash } from 'lucide-svelte'

	export let components: (BaseAppComponent & ButtonComponent)[]
	export let id: string

	const { selectedComponent, app, errorByComponent } =
		getContext<AppViewerContext>('AppViewerContext')

	function addComponent() {
		const actionId = getNextId(components.map((x) => x.id.split('_')[1]))

		const newComponent = {
			...appComponentFromType('buttoncomponent')(`${id}_${actionId}`),
			recomputeIds: []
		}
		components = [...components, newComponent]
		$app = $app
	}

	function deleteComponent(cid: string) {
		components = components.filter((x) => x.id !== cid)

		delete $errorByComponent[cid]

		$selectedComponent = [id]
		$app = $app
	}
</script>

<PanelSection title={`Menu items`}>
	{#if components.length == 0}
		<span class="text-xs text-tertiary">No action buttons</span>
	{/if}
	{#each components as component}
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div
			class={classNames(
				'w-full text-xs font-bold gap-1 truncate py-1.5 px-2 cursor-pointer transition-all justify-between flex items-center border border-gray-3 rounded-md',
				'bg-surface border-gray-300  hover:bg-gray-100 focus:bg-gray-100 text-secondary',
				$selectedComponent?.includes(component.id) ? 'outline outline-blue-500 bg-red-400' : ''
			)}
			on:click={() => {
				$selectedComponent = [component.id]
			}}
			on:keypress
		>
			<Badge color="dark-indigo">
				{component.id}
			</Badge>

			<div>
				{#if component.type == 'buttoncomponent'}
					Button
				{/if}
			</div>
			<div>
				<Button
					variant="border"
					color="red"
					on:click={() => deleteComponent(component.id)}
					startIcon={{ icon: Trash }}
					iconOnly
				/>
			</div>
		</div>
	{/each}
	<div class="w-full flex gap-2">
		<Button
			btnClasses="gap-1 flex items-center text-sm text-tertiary"
			wrapperClasses="w-full"
			color="light"
			variant="border"
			size="xs"
			on:click={() => addComponent()}
			title="Add Button"
			startIcon={{ icon: Plus }}
		>
			Add
		</Button>
	</div>
</PanelSection>
