<script lang="ts">
	import { Tabs, Tab, TabContent } from '$lib/components/common'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { CalendarCheck2, Terminal, Webhook } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
</script>

<main class="h-screen w-full">
	<div class="flex flex-col h-full">
		<slot name="header" />
		<SplitPanesWrapper>
			<Splitpanes>
				<Pane size={70} minSize={20} maxSize={70}>
					<slot name="form" />
				</Pane>
				<Pane size={30} minSize={20}>
					<Splitpanes horizontal class="h-full">
						<Pane size={100}>
							<Tabs selected="details">
								<Tab value="saved_inputs">Saved inputs</Tab>
								<Tab value="details">Code & Triggers</Tab>
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
													<Tabs selected="webhooks">
														<Tab value="webhooks">
															<span class="flex flex-row gap-2 items-center">
																<Webhook size={14} />
																Webhooks
															</span>
														</Tab>
														<Tab value="schedule">
															<span class="flex flex-row gap-2 items-center">
																<CalendarCheck2 size={14} />
																Schedule
															</span>
														</Tab>
														<Tab value="cli">
															<span class="flex flex-row gap-2 items-center">
																<Terminal size={14} />
																CLI
															</span>
														</Tab>

														<svelte:fragment slot="content">
															<TabContent value="webhooks">
																<slot name="webhooks" />
															</TabContent>
															<TabContent value="schedule">
																<slot name="schedule" />
															</TabContent>
															<TabContent value="cli">
																<slot name="cli" />
															</TabContent>
														</svelte:fragment>
													</Tabs>
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
