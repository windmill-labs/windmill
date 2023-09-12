<script lang="ts">
	import { Button, Drawer, DrawerContent, Badge } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { getContext } from 'svelte'
	import { Icon } from 'svelte-awesome'
	import { faAdd, faClose, faMagicWandSparkles } from '@fortawesome/free-solid-svg-icons'
	import { capitalize, classNames } from '$lib/utils'
	import { APP_TO_ICON_COMPONENT } from '../icons'
	import { charsToNumber, numberToChars } from '../flows/idUtils'
	import type { FlowCopilotContext } from './flow'
	import Alert from '../common/alert/Alert.svelte'
	import type { FlowEditorContext } from '../flows/types'
	import type { FlowModule } from '$lib/gen'

	export let getHubCompletions: (text: string, idx: number, type: 'trigger' | 'script') => void
	export let genFlow: (index: number, modules: FlowModule[], stepOnly?: boolean) => void
	export let flowCopilotMode: 'trigger' | 'sequence'

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	const { drawerStore, modulesStore, currentStepStore } =
		getContext<FlowCopilotContext>('FlowCopilotContext')
</script>

<Drawer bind:this={$drawerStore}>
	<DrawerContent on:close={$drawerStore.closeDrawer} title="AI Flow Builder">
		<div class="flex flex-col gap-6">
			{#if $flowStore.value.modules.length > 0 && $currentStepStore === undefined}
				<Alert type="error" title="Flow not empty">All flow steps will be overriden</Alert>
			{/if}
			<ToggleButtonGroup
				bind:selected={flowCopilotMode}
				on:selected={() => {
					if ($currentStepStore !== undefined) {
						$currentStepStore = numberToChars(0)
					}
				}}
			>
				<ToggleButton value="trigger" label="Trigger" />
				<ToggleButton value="sequence" label="Sequence" />
			</ToggleButtonGroup>
			{#each $modulesStore as copilotModule, i}
				<div>
					{#if i === 1 && $modulesStore[i - 1].type === 'trigger'}
						<div class="flex flex-row items-center mb-4 gap-1">
							<p class="text-sm font-semibold">For loop</p>
							<Badge color="indigo">{copilotModule.id}_loop</Badge>
						</div>
					{/if}
					<div class={i === 1 && $modulesStore[i - 1].type === 'trigger' ? 'pl-4' : ''}>
						<div class="flex flex-row items-center justify-between">
							<div class="flex flex-row justify-between items-center w-full mb-2">
								<div class="flex flex-row items-center gap-1">
									<p class="text-sm font-semibold"
										>{copilotModule.type === 'trigger' ? 'Trigger' : 'Action'}</p
									>
									<Badge color="indigo">{copilotModule.id}</Badge>
								</div>
								{#if flowCopilotMode === 'sequence' && i >= 1}
									<button
										on:click={() => {
											if ($currentStepStore !== undefined) {
												$currentStepStore = numberToChars(i < $modulesStore.length - 1 ? i : i - 1)
											}
											modulesStore.update((prev) => {
												prev.splice(i, 1)
												prev.forEach((m, idx) => {
													m.id = numberToChars(idx)
												})
												return prev
											})
										}}
									>
										<Icon data={faClose} />
									</button>
								{/if}
							</div>
						</div>
						{#if copilotModule.source !== undefined}
							<div
								class={classNames(
									'p-4 gap-4 flex flex-row grow  transition-all items-center rounded-md justify-between border',
									$currentStepStore !== undefined && i < charsToNumber($currentStepStore)
										? 'bg-gray-700/10'
										: 'bg-surface'
								)}
							>
								<div class="flex items-center gap-4">
									<div
										class="rounded-md p-1 flex justify-center items-center bg-surface border h-6 w-6"
									>
										{#if copilotModule.source === 'hub' && copilotModule.selectedCompletion}
											<svelte:component
												this={APP_TO_ICON_COMPONENT[copilotModule.selectedCompletion['app']]}
											/>
										{:else}
											<Icon data={faMagicWandSparkles} />
										{/if}
									</div>

									<div class="w-full text-left font-normal">
										<div class="text-primary flex-wrap text-sm font-medium">
											{copilotModule.source === 'hub' && copilotModule.selectedCompletion
												? copilotModule.selectedCompletion.summary
												: `Generate "${copilotModule.description}" in ${
														copilotModule.lang === 'bun' ? 'TypeScript' : 'Python'
												  }`}
										</div>
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
										if ($currentStepStore !== undefined && i < charsToNumber($currentStepStore)) {
											$currentStepStore = numberToChars(i)
										}
									}}
								>
									<Icon data={faClose} />
								</button>
							</div>
							{#if $currentStepStore !== undefined && i < charsToNumber($currentStepStore)}
								<p class="font-semibold text-sm text-green-600"
									>Already generated, edit step to regenerate from this point</p
								>
							{/if}
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
										getHubCompletions(copilotModule.description, i, copilotModule.type)
									} else {
										copilotModule.hubCompletions = []
									}
								}}
							/>
						{/if}
						{#if copilotModule.description.length > 2 && copilotModule.source === undefined}
							<div class="divide-y border rounded-md transition-all mt-2">
								<button
									class="p-4 gap-4 flex flex-row hover:bg-surface-hover bg-surface transition-all items-center rounded-md justify-between w-full"
									on:click={() => {
										copilotModule.source = 'custom'
										copilotModule.selectedCompletion = undefined
										copilotModule.lang = 'bun'
									}}
								>
									<div class="flex items-center gap-4">
										<div
											class="rounded-md p-1 flex justify-center items-center bg-surface border w-6 h-6"
										>
											<Icon data={faMagicWandSparkles} />
										</div>

										<div class="w-full text-left text-sm">
											<div class="text-primary flex-wrap font-medium">
												Generate "{copilotModule.description}" in TypeScript
											</div>
										</div>
									</div>
								</button>
								<button
									class="p-4 gap-4 flex flex-row hover:bg-surface-hover bg-surface transition-all items-center rounded-md justify-between w-full"
									on:click={() => {
										copilotModule.source = 'custom'
										copilotModule.selectedCompletion = undefined
										copilotModule.lang = 'python3'
									}}
								>
									<div class="flex items-center gap-4">
										<div
											class="rounded-md p-1 flex justify-center items-center bg-surface border w-6 h-6"
										>
											<Icon data={faMagicWandSparkles} />
										</div>

										<div class="w-full text-left text-sm">
											<div class="text-primary flex-wrap font-medium">
												Generate "{copilotModule.description}" in Python
											</div>
										</div>
									</div>
								</button>
							</div>
							{#if copilotModule.hubCompletions.length > 0}
								<p class="mt-2 font-semibold text-sm">Hub scripts</p>
								<ul class="divide-y border rounded-md transition-all mt-1">
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
														class="rounded-md p-1 flex justify-center items-center bg-surface border w-6 h-6"
													>
														<svelte:component this={APP_TO_ICON_COMPONENT[item['app']]} />
													</div>

													<div class="text-left font-normal text-sm">
														<div class="text-primary font-medium">
															{item.summary ?? ''}
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
									selectedCompletion: undefined,
									lang: undefined
								}
							])}>Add step</Button
					>
				</div>
			{/if}

			<Button
				on:click={() =>
					$currentStepStore !== undefined
						? genFlow(charsToNumber($currentStepStore), $flowStore.value.modules)
						: genFlow(0, $flowStore.value.modules)}
				spacingSize="md"
				startIcon={{ icon: faMagicWandSparkles }}
				disabled={$modulesStore.find((m) => m.source === undefined) !== undefined}
			>
				{$currentStepStore !== undefined
					? `Regenerate flow from step '${$currentStepStore}'`
					: 'Build flow'}
			</Button>
		</div>
	</DrawerContent>
</Drawer>
