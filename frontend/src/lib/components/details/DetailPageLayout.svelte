<script lang="ts">
	import { Tabs, Tab, TabContent } from '$lib/components/common'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { CalendarCheck2, Terminal, Webhook } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'

	let triggerSelected: 'webhooks' | 'schedule' | 'cli' = 'webhooks'

	export let isOperator: boolean = false
</script>

<main class="h-screen w-full">
	<div class="flex flex-col h-full">
		<slot name="header" />
		<SplitPanesWrapper>
			<Splitpanes>
				<Pane size={65} minSize={50}>
					<slot name="form" />
				</Pane>
				<Pane size={35} minSize={15}>
					<Splitpanes horizontal class="h-full">
						<Pane size={100}>
							<Tabs selected="saved_inputs">
								<Tab value="saved_inputs">Saved inputs</Tab>
								{#if !isOperator}
									<Tab value="details">Details & Triggers</Tab>
								{/if}
								<svelte:fragment slot="content">
									<div class="overflow-hidden" style="height:calc(100% - 32px);">
										<TabContent value="saved_inputs" class="flex flex-col flex-1 h-full">
											<slot name="save_inputs" />
										</TabContent>
										<TabContent value="details" class="flex flex-col flex-1 h-full">
											<Splitpanes horizontal class="h-full">
												<Pane size={50} minSize={20}>
													<slot name="details" />
												</Pane>
												<Pane size={50} minSize={20}>
													<Tabs bind:selected={triggerSelected}>
														<Tab value="webhooks">
															<span class="flex flex-row gap-2 items-center">
																<Webhook size={14} />
																Webhooks
															</span>
														</Tab>
														<Tab value="schedule">
															<span class="flex flex-row gap-2 items-center">
																<CalendarCheck2 size={14} />
																Schedules
															</span>
														</Tab>
														<Tab value="cli">
															<span class="flex flex-row gap-2 items-center">
																<Terminal size={14} />
																CLI
															</span>
														</Tab>
													</Tabs>

													<div class="h-[calc(100%-32px)]">
														<div class="h-full overflow-auto">
															{#if triggerSelected === 'webhooks'}
																<slot name="webhooks" />
															{:else if triggerSelected === 'schedule'}
																<slot name="schedule" />
															{:else if triggerSelected === 'cli'}
																<slot name="cli" />
															{/if}
														</div>
													</div>
												</Pane>
											</Splitpanes>
										</TabContent>
									</div>
								</svelte:fragment>
							</Tabs>
						</Pane>
					</Splitpanes>
				</Pane>
			</Splitpanes>
		</SplitPanesWrapper>
	</div>
</main>
