<script lang="ts">
	import { scriptPathToHref } from '$lib/utils'

	import { slide } from 'svelte/transition'

	import InputTransformsViewer from './InputTransformsViewer.svelte'
	import IconedPath from './IconedPath.svelte'
	import type { FlowModule } from '$lib/gen'
	import HighlightCode from './HighlightCode.svelte'
	import { faBug } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'

	export let modules: FlowModule[]
	export let failureModule: FlowModule | undefined

	let open: { [id: number]: boolean } = {}
</script>

<p class="font-black text-lg my-6 w-full">
	<span>{modules?.length} Step{modules?.length > 1 ? 's' : ''} </span>
	<span class="mt-4" />
</p>
<ul class="-mb-8 w-full">
	{#each modules ?? [] as mod, i}
		<li class="w-full">
			<div class="relative pb-8 w-full">
				{#if i < (modules ?? []).length - 1 || failureModule}
					<span class="absolute top-4 left-4 -ml-px h-full w-0.5 bg-gray-200" aria-hidden="true" />
				{/if}
				<div class="relative flex space-x-3">
					<div>
						<span
							class="h-8 w-8 rounded-full bg-blue-600 flex items-center justify-center ring-8 ring-white text-white"
						>
							{i + 1}
						</span>
					</div>
					<div class="min-w-0 flex-1 pt-1.5 flex justify-between space-x-4 w-full">
						<div class="w-full">
							<span class="text-black">{mod?.summary ?? ''}</span>
							<p class="text-sm text-gray-500">
								{#if mod?.value?.type == 'script'}
									<a
										target="_blank"
										href={scriptPathToHref(mod?.value?.path ?? '')}
										class="font-medium text-gray-900"
									>
										<IconedPath path={mod?.value?.path ?? ''} />
									</a>
									{#if mod?.value?.path?.startsWith('hub/')}
										<div>
											<button
												on:click={async () => {
													open[i] = !open[i]
												}}
												class="mb-2 underline text-black"
											>
												View code and inputs {open[i] ? '(-)' : '(+)'}</button
											>
											{#if open[i]}
												<div class="border border-black p-2 bg-gray-50  divide-y">
													<InputTransformsViewer inputTransforms={mod?.input_transforms} />
													<div class="w-full h-full mt-6">
														<iframe
															style="height: 400px;"
															class="w-full h-full  text-sm"
															title="embedded script from hub"
															frameborder="0"
															src="https://hub.windmill.dev/embed/script/{mod?.value?.path?.substring(
																4
															)}"
														/>
													</div>
												</div>
											{/if}
										</div>
									{/if}
								{:else if mod?.value?.type == 'rawscript'}
									<button on:click={() => (open[i] = !open[i])} class="mb-2 underline text-black">
										Raw {mod?.value?.language} script {open[i] ? '(-)' : '(+)'}</button
									>

									{#if open[i]}
										<div transition:slide class="border border-black p-2 bg-gray-50 w-full">
											<InputTransformsViewer inputTransforms={mod?.input_transforms} />
											<HighlightCode
												language={mod?.value?.language ?? 'deno'}
												code={mod?.value?.content}
											/>
										</div>
									{/if}
								{:else if mod?.value?.type == 'flow'}
									Flow at path {mod?.value?.path}
								{:else if mod?.value?.type == 'forloopflow'}
									For loop over all the elements of the list returned as a result of step {i}:
									<svelte:self modules={mod.value.modules} />
								{/if}
							</p>
						</div>
					</div>
				</div>
			</div>
		</li>
	{/each}
	{#if failureModule}
		<li class="w-full">
			<div class="relative pb-8 w-full">
				<div class="relative flex space-x-3">
					<div>
						<span
							class="h-8 w-8 rounded-full bg-red-600 flex items-center justify-center ring-8 ring-white text-white"
						>
							<Icon data={faBug} />
						</span>
					</div>
					<div class="min-w-0 flex-1 pt-1.5 flex justify-between space-x-4 w-full">
						<div class="w-full">
							<span class="text-black">{failureModule?.summary ?? ''}</span>
							<p class="text-sm text-gray-500">
								{#if failureModule?.value?.type == 'script'}
									<a
										target="_blank"
										href={scriptPathToHref(failureModule?.value?.path ?? '')}
										class="font-medium text-gray-900"
									>
										<IconedPath path={failureModule?.value?.path ?? ''} />
									</a>
									{#if failureModule?.value?.path?.startsWith('hub/')}
										<div>
											<button
												on:click={async () => {
													open[modules.length] = !open[modules.length]
												}}
												class="mb-2 underline text-black"
											>
												View code and inputs {open[modules.length] ? '(-)' : '(+)'}</button
											>
											{#if open[modules.length]}
												<div class="border border-black p-2 bg-gray-50  divide-y">
													<InputTransformsViewer
														inputTransforms={failureModule?.input_transforms}
													/>
													<div class="w-full h-full mt-6">
														<iframe
															style="height: 400px;"
															class="w-full h-full  text-sm"
															title="embedded script from hub"
															frameborder="0"
															src="https://hub.windmill.dev/embed/script/{failureModule?.value?.path?.substring(
																4
															)}"
														/>
													</div>
												</div>
											{/if}
										</div>
									{/if}
								{:else if failureModule?.value?.type == 'rawscript'}
									<button
										on:click={() => (open[modules.length] = !open[modules.length])}
										class="mb-2 underline text-black"
									>
										Error handler: Raw {failureModule?.value?.language} script {open[modules.length]
											? '(-)'
											: '(+)'}</button
									>

									{#if open[modules.length]}
										<div transition:slide class="border border-black p-2 bg-gray-50 w-full">
											<InputTransformsViewer inputTransforms={failureModule?.input_transforms} />
											<HighlightCode
												language={failureModule?.value?.language ?? 'deno'}
												code={failureModule?.value?.content}
											/>
										</div>
									{/if}
								{/if}
							</p>
						</div>
					</div>
				</div>
			</div>
		</li>
	{/if}
</ul>
