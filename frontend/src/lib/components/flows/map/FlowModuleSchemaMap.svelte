<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import Icon from 'svelte-awesome'
	import { faCalendarAlt, faPen, faPlus, faSliders } from '@fortawesome/free-solid-svg-icons'
	import { emptyFlowModuleState, isEmptyFlowModule } from '$lib/components/flows/flowStateUtils'
	import { classNames, emptyModule } from '$lib/utils'
	import type { FlowModuleState } from '../flowState'
	import type { FlowModule } from '$lib/gen'

	import FlowErrorHandlerItem from './FlowErrorHandlerItem.svelte'
	import RemoveStepConfirmationModal from '../content/RemoveStepConfirmationModal.svelte'

	export let prefix: string | undefined = undefined
	export let modules: FlowModule[]
	export let moduleStates: FlowModuleState[]

	const { select, selectedId, schedule } = getContext<FlowEditorContext>('FlowEditorContext')

	function insertAtIndex(index: number): void {
		moduleStates.splice(index, 0, emptyFlowModuleState())
		modules.splice(index, 0, emptyModule())
		moduleStates = moduleStates
		modules = modules
		select([prefix, String(index)].filter(Boolean).join('-'))
	}

	function removeAtIndex(index: number): void {
		select('settings')

		modules.splice(index, 1)
		moduleStates.splice(index, 1)
		moduleStates = moduleStates
		modules = modules
	}
	let indexToRemove: number | undefined = undefined
	$: confirmationModalOpen = indexToRemove !== undefined
</script>

<div class="flex flex-col justify-between h-full">
	<ul class="w-full">
		{#if prefix === undefined}
			<div
				on:click={() => select('settings')}
				class={classNames(
					'border w-full rounded-md p-2 bg-white text-sm cursor-pointer flex items-center mb-4',
					$selectedId.includes('settings')
						? 'outline outline-offset-1 outline-2  outline-slate-900'
						: ''
				)}
			>
				<Icon data={faSliders} class="mr-2" />
				<span class="font-bold flex flex-row justify-between w-full flex-wrap gap-2"
					>Settings <span
						class={classNames('badge', $schedule?.enabled ? 'badge-on' : 'badge-off')}
						on:click|stopPropagation={() => select('settings-schedule')}
					>
						{$schedule.cron}
						<Icon class={$schedule.cron ? 'ml-2' : ''} data={faCalendarAlt} scale={0.8} />
					</span></span
				>
			</div>

			<FlowModuleSchemaItem
				on:click={() => select('inputs')}
				isFirst
				hasLine
				selected={$selectedId === 'inputs'}
			>
				<div slot="icon">
					<Icon data={faPen} scale={0.8} />
				</div>
				<div slot="content" class="flex flex-row">
					<span>Inputs</span>
				</div>
			</FlowModuleSchemaItem>
		{/if}

		{#each modules as mod, index (index)}
			<button
				on:click={() => insertAtIndex(index)}
				type="button"
				class="text-gray-900 m-0.5 my-0.5 bg-white border border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
			>
				<Icon data={faPlus} scale={0.8} />
			</button>
			{#if mod.value.type === 'forloopflow'}
				<li>
					<FlowModuleSchemaItem
						deletable
						on:delete={() => removeAtIndex(index)}
						on:click={() => select(['loop', String(index)].join('-'))}
						selected={$selectedId === ['loop', String(index)].join('-')}
						retry={mod.retry?.constant != undefined || mod.retry?.exponential != undefined}
						earlyStop={mod.stop_after_if != undefined}
						suspend={Boolean(mod.suspend)}
					>
						<div slot="icon">
							<span>{index + 1}</span>
						</div>
						<div slot="content" class="truncate block w-full">
							<span>{mod.summary || 'For loop'}</span>
						</div>
					</FlowModuleSchemaItem>

					<div class="flex text-xs">
						<div class="line mr-2 w-8" />

						<div class="w-full mb-2">
							{#if moduleStates[index]?.childFlowModules}
								<svelte:self
									prefix={String(index)}
									moduleStates={moduleStates[index].childFlowModules}
									modules={mod.value.modules}
								/>
							{/if}
						</div>
					</div>

					<div class="italic text-xs pl-10 text-gray-600">
						The loop's result is the list of every iteration's result.
					</div>
				</li>
			{:else}
				<li>
					<FlowModuleSchemaItem
						on:click={() => select([prefix, String(index)].filter(Boolean).join('-'))}
						color={prefix ? 'orange' : 'blue'}
						isLast={index === modules.length - 1}
						selected={$selectedId === [prefix, String(index)].filter(Boolean).join('-')}
						deletable
						on:delete={(event) => {
							if (event.detail.event.shiftKey || isEmptyFlowModule(mod)) {
								removeAtIndex(index)
							} else {
								indexToRemove = index
							}
						}}
						retry={mod.retry?.constant != undefined || mod.retry?.exponential != undefined}
						earlyStop={mod.stop_after_if != undefined}
						suspend={Boolean(mod.suspend)}
					>
						<div slot="icon">
							<span>{index + 1}</span>
						</div>
						<div slot="content" class="w-full truncate block">
							<span
								>{mod.summary ||
									mod.value.path ||
									(mod.value.type === 'rawscript'
										? `Inline ${mod.value.language}`
										: 'Select a script')}</span
							>
						</div>
					</FlowModuleSchemaItem>
				</li>
			{/if}
		{/each}

		<button
			on:click={() => insertAtIndex(modules.length)}
			type="button"
			class="text-gray-900 bg-white border m-0.5 border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
		>
			<Icon data={faPlus} scale={0.8} />
		</button>
	</ul>
	{#if prefix === undefined}
		<FlowErrorHandlerItem />
	{/if}
</div>

<RemoveStepConfirmationModal
	bind:open={confirmationModalOpen}
	on:canceled={() => {
		indexToRemove = undefined
	}}
	on:confirmed={() => {
		if (indexToRemove !== undefined) {
			removeAtIndex(indexToRemove)
			indexToRemove = undefined
		}
	}}
/>

<style>
	.badge {
		@apply whitespace-nowrap text-sm font-medium border px-2.5 py-0.5 rounded cursor-pointer flex items-center;
	}

	.badge-on {
		@apply bg-blue-100 text-blue-800 hover:bg-blue-200;
	}

	.badge-off {
		@apply bg-gray-100 text-gray-800 hover:bg-gray-200;
	}
</style>
