<script lang="ts">
	import { Button, Drawer, DrawerContent, Badge } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { getContext } from 'svelte'
	import { capitalize, classNames } from '$lib/utils'
	import { APP_TO_ICON_COMPONENT } from '../icons'
	import { charsToNumber, numberToChars } from '../flows/idUtils'
	import type { FlowCopilotContext } from './flow'
	import Alert from '../common/alert/Alert.svelte'
	import type { FlowEditorContext } from '../flows/types'
	import type { FlowModule } from '$lib/gen'
	import { Plus, Wand2, X } from 'lucide-svelte'

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
				<ToggleButton value="sequence" label="Sequence" />
				<ToggleButton value="trigger" label="Trigger" />
			</ToggleButtonGroup>
			{#if flowCopilotMode === 'trigger'}
				<Alert title="Trigger flow builder">
					Build a flow with two scripts, one that regularly checks for changes in an external system
					and a second that is executed for each change using a for-loop. For both steps, you can
					either choose a script from the hub or generate one from scratch using Windmill AI. The
					inputs of the for-loop action are automatically filled in with the ouputs of the trigger
					step. At the end of the process, flow inputs are inferred and you just need to fill them
					in. <br /><br />
					The flow is automatically set to run every 15 minutes when deployed.
				</Alert>{:else}
				<Alert title="Sequence flow builder">
					Build a flow with a sequence of scripts that are executed one after the other. For each
					step, you can either choose a script from the hub or generate one from scratch using
					Windmill AI. Each step inputs are automatically filled in with the previous step's
					outputs. At the end of the process, flow inputs are inferred and you just need to fill
					them in.
				</Alert>
			{/if}
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
										<X />
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
											<Wand2 />
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
										>{capitalize(copilotModule.selectedCompletion.kind.toString())}</Badge
									>
								{/if}

								<button
									on:click={() => {
										copilotModule = {
											...copilotModule,
											selectedCompletion: undefined,
											source: undefined,
											lang: undefined
										}
										if ($currentStepStore !== undefined && i < charsToNumber($currentStepStore)) {
											$currentStepStore = numberToChars(i)
										}
									}}
								>
									<X />
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
									? 'describe what should trigger your flow e.g. "new slack message"'
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
										copilotModule = {
											...copilotModule,
											source: 'custom',
											selectedCompletion: undefined,
											lang: 'bun'
										}
									}}
								>
									<div class="flex items-center gap-4">
										<div
											class="rounded-md p-1 flex justify-center items-center bg-surface border w-6 h-6"
										>
											<Wand2 />
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
										copilotModule = {
											...copilotModule,
											source: 'custom',
											selectedCompletion: undefined,
											lang: 'python3'
										}
									}}
								>
									<div class="flex items-center gap-4">
										<div
											class="rounded-md p-1 flex justify-center items-center bg-surface border w-6 h-6"
										>
											<Wand2 />
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
													copilotModule = {
														...copilotModule,
														source: 'hub',
														selectedCompletion: item,
														lang: undefined
													}
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
													<Badge color="gray" baseClass="border"
														>{capitalize(item.kind.toString())}</Badge
													>
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
						startIcon={{ icon: Plus }}
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
				startIcon={{ icon: Wand2 }}
				disabled={$modulesStore.find((m) => m.source === undefined) !== undefined}
			>
				{$currentStepStore !== undefined
					? `Regenerate flow from step '${$currentStepStore}'`
					: 'Build flow'}
			</Button>
		</div>
	</DrawerContent>
</Drawer>
