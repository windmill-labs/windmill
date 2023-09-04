<script lang="ts">
	import { Button, Drawer, DrawerContent, Badge } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { getContext } from 'svelte'
	import { Icon } from 'svelte-awesome'
	import { faAdd, faClose, faMagicWandSparkles } from '@fortawesome/free-solid-svg-icons'
	import { capitalize, classNames } from '$lib/utils'
	import { APP_TO_ICON_COMPONENT } from '../icons'
	import { numberToChars } from '../flows/idUtils'
	import type { FlowCopilotContext } from './flow'

	export let hubCompletions: (text: string, idx: number, type: 'trigger' | 'script') => void
	export let genFlow: (index: number) => void
	export let flowCopilotMode: 'trigger' | 'sequence'

	const { drawerStore, modulesStore } = getContext<FlowCopilotContext>('FlowCopilotContext')
</script>

<Drawer bind:this={$drawerStore}>
	<DrawerContent on:close={$drawerStore.closeDrawer}>
		<h1 class="pb-4">AI Flow Builder</h1>
		<div class="flex flex-col gap-4">
			<ToggleButtonGroup bind:selected={flowCopilotMode}>
				<ToggleButton value="trigger" label="Trigger" />
				<ToggleButton value="sequence" label="Sequence" />
			</ToggleButtonGroup>
			{#each $modulesStore as copilotModule, i}
				<div>
					{#if i === 1 && $modulesStore[i - 1].type === 'trigger'}
						<div class="flex flex-row items-center pb-2 gap-2">
							<Badge color="indigo">{numberToChars(i)}_loop</Badge>
							<p class="font-semibold">For loop</p>
						</div>
					{/if}
					<div class={i === 1 && $modulesStore[i - 1].type === 'trigger' ? 'pl-4' : ''}>
						<div class="flex flex-row items-center justify-between">
							<div class="flex flex-row items-center gap-2">
								<Badge color="indigo">{numberToChars(i)}</Badge>
								<p class="font-semibold"
									>{copilotModule.type === 'trigger' ? 'Trigger' : 'Action'}</p
								>
							</div>
							{#if flowCopilotMode === 'sequence' && i >= 1}
								<button
									on:click={() => {
										modulesStore.update((prev) => {
											prev.splice(i, 1)
											return prev
										})
									}}
								>
									<Icon data={faClose} />
								</button>
							{/if}
						</div>
						{#if copilotModule.source !== undefined}
							<div
								class="p-4 gap-4 flex flex-row grow bg-surface transition-all items-center rounded-md justify-between border"
							>
								<div class="flex items-center gap-4">
									<div
										class={classNames(
											'rounded-md p-1 flex justify-center items-center border',
											'bg-surface border'
										)}
									>
										{#if copilotModule.source === 'hub' && copilotModule.selectedCompletion}
											<svelte:component
												this={APP_TO_ICON_COMPONENT[copilotModule.selectedCompletion['app']]}
												height={18}
												width={18}
											/>
										{:else}
											<Icon data={faMagicWandSparkles} />
										{/if}
									</div>

									<div class="w-full text-left font-normal">
										<div class="text-primary flex-wrap text-md font-semibold mb-1">
											{copilotModule.source === 'hub' && copilotModule.selectedCompletion
												? copilotModule.selectedCompletion.summary
												: copilotModule.description}
										</div>
										{#if copilotModule.source === 'hub' && copilotModule.selectedCompletion}
											<div class="text-secondary text-xs break-all">
												{copilotModule.selectedCompletion.path}
											</div>
										{/if}
									</div>
								</div>
								{#if copilotModule.source === 'hub' && copilotModule.selectedCompletion && copilotModule.selectedCompletion?.kind !== 'script'}
									<Badge color="gray" baseClass="border"
										>{capitalize(copilotModule.selectedCompletion.kind)}</Badge
									>
								{/if}

								<button
									on:click={() => {
										copilotModule.selectedCompletion = undefined
										copilotModule.source = undefined
									}}
								>
									<Icon data={faClose} />
								</button>
							</div>
						{:else}
							<input
								name="description"
								type="text"
								placeholder={copilotModule.type === 'trigger'
									? 'describe what should trigger your flow'
									: 'describe what this step should do'}
								bind:value={copilotModule.description}
								on:input={() => {
									if (copilotModule.description.length > 2) {
										hubCompletions(copilotModule.description, i, copilotModule.type)
									} else {
										copilotModule.hubCompletions = []
									}
								}}
							/>
						{/if}
						{#if copilotModule.description.length > 3 && copilotModule.source === undefined}
							<ul class="divide-y border rounded-md transition-all mt-2">
								<li>
									<button
										class="p-4 gap-4 flex flex-row hover:bg-surface-hover bg-surface transition-all items-center rounded-md justify-between w-full"
										on:click={() => {
											copilotModule.source = 'custom'
											copilotModule.selectedCompletion = undefined
										}}
									>
										<div class="flex items-center gap-4">
											<div
												class={classNames(
													'rounded-md p-1 flex justify-center items-center border',
													'bg-surface border'
												)}
											>
												<Icon data={faMagicWandSparkles} />
											</div>

											<div class="text-left font-normal">
												<div class="text-primary flex-wrap text-md font-semibold mb-1">
													Generate step from scratch using Windmill AI
												</div>
											</div>
										</div>
									</button>
								</li>
								{#each copilotModule.hubCompletions as item (item.path)}
									<li>
										<button
											class="p-4 gap-4 flex flex-row hover:bg-surface-hover bg-surface transition-all items-center rounded-md justify-between w-full"
											on:click={() => {
												copilotModule.source = 'hub'
												copilotModule.selectedCompletion = item
											}}
										>
											<div class="flex items-center gap-4">
												<div
													class={classNames(
														'rounded-md p-1 flex justify-center items-center border',
														'bg-surface border'
													)}
												>
													<svelte:component
														this={APP_TO_ICON_COMPONENT[item['app']]}
														height={18}
														width={18}
													/>
												</div>

												<div class="text-left font-normal">
													<div class="text-primary text-md font-semibold mb-1">
														{item.summary ?? ''}
													</div>
													<div class="text-secondary text-xs break-all">
														{item.path}
													</div>
												</div>
											</div>
											{#if item.kind !== 'script'}
												<Badge color="gray" baseClass="border">{capitalize(item.kind)}</Badge>
											{/if}
										</button>
									</li>
								{/each}
							</ul>
						{/if}
					</div>
				</div>
			{/each}
			{#if flowCopilotMode !== 'trigger'}
				<div class="flex justify-start">
					<Button
						startIcon={{ icon: faAdd }}
						size="xs"
						variant="border"
						on:click={() =>
							modulesStore.update((prev) => [
								...prev,
								{
									id: numberToChars(prev.length),
									type: 'script',
									description: '',
									code: '',
									source: undefined,
									hubCompletions: [],
									selectedCompletion: undefined
								}
							])}>Add step</Button
					>
				</div>
			{/if}

			<Button
				on:click={() => genFlow(0)}
				spacingSize="md"
				startIcon={{ icon: faMagicWandSparkles }}
				disabled={$modulesStore.find((m) => m.source === undefined) !== undefined}
			>
				Build flow
			</Button>
		</div>
	</DrawerContent>
</Drawer>
