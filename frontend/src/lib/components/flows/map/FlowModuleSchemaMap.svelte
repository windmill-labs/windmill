<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import Icon from 'svelte-awesome'
	import { faBug, faPen, faPlus, faSliders } from '@fortawesome/free-solid-svg-icons'
	import {
		emptyFlowModuleSchema,
		NEVER_TESTED_THIS_FAR
	} from '$lib/components/flows/flowStateUtils'
	import { classNames, emptyModule, emptySchema } from '$lib/utils'
	import { flowStateStore, type FlowModuleState } from '../flowState'
	import type { FlowModule } from '$lib/gen'
	import Toggle from '$lib/components/Toggle.svelte'
	import { flowStore } from '../flowStore'

	export let prefix: string | undefined = undefined
	export let modules: FlowModule[]
	export let moduleStates: FlowModuleState[]

	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	function insertAtIndex(index: number): void {
		moduleStates.splice(index, 0, emptyFlowModuleSchema())
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
				<span class="font-bold">Settings</span>
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
				<div slot="content">
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
					>
						<div slot="icon">
							<span>{index + 1}</span>
						</div>
						<div slot="content">
							<span>For loop</span>
						</div>
					</FlowModuleSchemaItem>

					<div class="flex text-xs">
						<div class="line mr-2 w-8" />

						<div class="w-full mb-2">
							<svelte:self
								prefix={String(index)}
								moduleStates={moduleStates[index].childFlowModules}
								modules={mod.value.modules}
							/>
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
						on:delete={() => removeAtIndex(index)}
					>
						<div slot="icon">
							<span>{index + 1}</span>
						</div>
						<div slot="content" class="w-full">
							<input
								bind:value={mod.summary}
								placeholder={mod.summary ||
									mod.value.path ||
									(mod.value.type === 'rawscript'
										? `Inline ${mod.value.language}`
										: 'Select a script')}
							/>
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
		<div
			on:click={() => {
				if ($flowStore.value.failure_module) {
					select('failure')
				}
			}}
			class={classNames(
				'border rounded-md p-2 bg-white text-sm cursor-pointer mt-4 flex flex-between  items-center',
				$selectedId.includes('failure')
					? 'outline outline-offset-1 outline-2  outline-slate-900'
					: ''
			)}
		>
			<div class="w-full">
				<Icon data={faBug} class="mr-2" />
				<span class="font-bold">Error handler</span>
			</div>
			<Toggle
				checked={Boolean($flowStore.value.failure_module)}
				on:change={() => {
					if ($flowStore.value.failure_module) {
						$flowStore.value.failure_module = undefined
						select('settings')
					} else {
						$flowStateStore.failureModule = {
							schema: emptySchema(),
							previewResult: NEVER_TESTED_THIS_FAR,
							childFlowModules: []
						}
						$flowStore.value.failure_module = emptyModule()
						select('failure')
					}
				}}
			/>
		</div>
	{/if}
</div>

<style>
	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, #bbb 4px 8px) 50%/1px 100%
			no-repeat;
	}
</style>
