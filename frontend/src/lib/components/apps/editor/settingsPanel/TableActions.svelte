<script lang="ts">
	import { Badge } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { getNextId } from '$lib/components/flows/idUtils'
	import { classNames } from '$lib/utils'
	import { faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import { Icon } from 'svelte-awesome'
	import type { AppViewerContext, BaseAppComponent } from '../../types'
	import {
		appComponentFromType,
		clearErrorByComponentId,
		clearJobsByComponentId
	} from '../appUtils'
	import type { ButtonComponent, CheckboxComponent, SelectComponent } from '../component'
	import PanelSection from './common/PanelSection.svelte'
	import { Inspect, List, ToggleRightIcon } from 'lucide-svelte'

	export let components: (BaseAppComponent &
		(ButtonComponent | CheckboxComponent | SelectComponent))[]
	export let id: string

	const { selectedComponent, app, errorByComponent, jobs } =
		getContext<AppViewerContext>('AppViewerContext')

	function addComponent(typ: 'buttoncomponent' | 'checkboxcomponent' | 'selectcomponent') {
		const actionId = getNextId(components.map((x) => x.id.split('_')[1]))

		const newComponent = {
			...appComponentFromType(typ)(`${id}_${actionId}`),
			recomputeIds: []
		}
		components = [...components, newComponent]
		$app = $app
	}

	function deleteComponent(cid: string) {
		components = components.filter((x) => x.id !== cid)

		$errorByComponent = clearErrorByComponentId(cid, $errorByComponent)
		$jobs = clearJobsByComponentId(cid, $jobs)

		$selectedComponent = [id]
		$app = $app
	}
</script>

<PanelSection title={`Table actions`}>
	{#if components.length == 0}
		<span class="text-xs text-gray-500">No action buttons</span>
	{/if}
	{#each components as component}
		<div
			class={classNames(
				'w-full text-xs font-bold gap-1 truncate py-1.5 px-2 cursor-pointer transition-all justify-between flex items-center border border-gray-3 rounded-md',
				'bg-white border-gray-300  hover:bg-gray-100 focus:bg-gray-100 text-gray-700',
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
				{:else if component.type == 'selectcomponent'}
					Select
				{:else if component.type == 'checkboxcomponent'}
					Toggle
				{/if}
			</div>
			<div>
				<Button variant="border" color="red" on:click={() => deleteComponent(component.id)}>
					<Icon class="h-3" data={faTrashAlt} />
				</Button>
			</div>
		</div>
	{/each}
	<div class="w-full flex gap-2">
		<Button
			btnClasses="gap-1 flex items-center text-sm text-gray-600"
			wrapperClasses="w-full"
			color="light"
			variant="border"
			on:click={() => addComponent('buttoncomponent')}
			title="Add Button"
		>
			+ <Inspect size={14} />
		</Button>
		<Button
			btnClasses="gap-1 flex items-center text-sm text-gray-600"
			wrapperClasses="w-full"
			color="light"
			variant="border"
			on:click={() => addComponent('checkboxcomponent')}
			title="Add Toggle"
		>
			+ <ToggleRightIcon size={14} />
		</Button>
		<Button
			btnClasses="gap-1 flex items-center text-sm text-gray-600"
			wrapperClasses="w-full"
			color="light"
			variant="border"
			on:click={() => addComponent('selectcomponent')}
			title="Add Select"
		>
			+ <List size={14} />
		</Button>
	</div>
</PanelSection>
