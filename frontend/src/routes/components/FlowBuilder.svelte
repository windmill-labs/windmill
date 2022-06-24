<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { onMount } from 'svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import { FlowService, type Flow } from '../../gen'
	import { clearPreviewResults, workspaceStore } from '../../stores'
	import { sendUserToast } from '../../utils'
	import FlowEditor from './FlowEditor.svelte'
	import Path from './Path.svelte'
	import Required from './Required.svelte'
	import ScriptSchema from './ScriptSchema.svelte'

	export let flow: Flow
	export let initialPath: string = ''

	$: step = Number($page.url.searchParams.get('step')) || 1

	async function saveFlow(): Promise<void> {
		try {
			if (initialPath == '') {
				await FlowService.createFlow({
					workspace: $workspaceStore!,
					requestBody: {
						path: flow.path,
						summary: flow.summary,
						description: flow.description ?? '',
						value: flow.value,
						schema: flow.schema
					}
				})
			} else {
				await FlowService.updateFlow({
					workspace: $workspaceStore!,
					path: initialPath,
					requestBody: {
						path: flow.path,
						summary: flow.summary,
						description: flow.description ?? '',
						value: flow.value,
						schema: flow.schema
					}
				})
			}
			sendUserToast(`Success! flow saved at ${flow.path}`)
			goto(`/flows/get/${flow.path}`)
		} catch (error) {
			if (error.status === 400) {
				sendUserToast(error.body, true)
			} else {
				sendUserToast(`Ooops.Something bad happened: ${error}`, true)
				console.error(error)
			}
		}
	}

	async function inferSchema() {
		//await inferArgs(flow.content, flow.schema);
	}

	async function changeStep(step: number) {
		goto(`?step=${step}`)
	}

	$: {
		$page.url.searchParams.set('state', btoa(JSON.stringify(flow)))
		history.replaceState({}, '', $page.url)
	}

	onMount(() => {
		clearPreviewResults()
	})
</script>

<div class="flex flex-col h-screen max-w-screen-lg xl:-ml-20 xl:pl-4 w-full -mt-4 pt-4 md:mx-10 ">
	<!-- Nav between steps-->
	<div class="flex flex-col w-full">
		<div class="justify-between flex flex-row drop-shadow-sm w-full">
			<div class="wizard-nav flex flex-row w-full">
				<button
					class="{step === 1
						? 'default-button-disabled text-gray-700'
						: 'default-button-secondary'} min-w-max ml-2"
					on:click={() => {
						changeStep(1)
					}}>Step 1: Metadata</button
				>
				<button
					class="{step === 2
						? 'default-button-disabled text-gray-700'
						: 'default-button-secondary'} min-w-max ml-2"
					on:click={() => {
						changeStep(2)
					}}>Step 2: Flow</button
				>
				<button
					class="{step === 3
						? 'default-button-disabled text-gray-700'
						: 'default-button-secondary'} min-w-max ml-2"
					on:click={() => {
						changeStep(3)
					}}>Step 3: UI customisation</button
				>
			</div>
			<div class="flex flex-row-reverse ml-2">
				{#if step != 3}
					<button
						disabled={step == 1 &&
							(flow.path == undefined || flow.path == '' || flow.path.split('/')[2] == '')}
						class="default-button px-6 max-h-8"
						on:click={() => {
							changeStep(step + 1)
						}}>Next</button
					>
					{#if step == 2}
						<button class="default-button-secondary px-6 max-h-8 mr-2" on:click={saveFlow}
							>Save</button
						>
					{/if}
				{:else}
					<button class="default-button px-6 self-end" on:click={saveFlow}>Save</button>
				{/if}
			</div>
		</div>
		<div class="flex flex-row-reverse">
			<span class="my-1 text-sm text-gray-500 italic">
				{#if initialPath && initialPath != flow.path} {initialPath} &rightarrow; {/if}
				{flow.path}
			</span>
		</div>
	</div>

	<!-- metadata -->
	{#if step === 1}
		<div class="grid grid-cols-1 gap-6 max-w-7xl">
			<Path bind:path={flow.path} {initialPath} namePlaceholder="example/my/flow">
				<div slot="ownerToolkit" class="text-gray-700 text-2xs">
					Flow permissions depend on their path. Select the group <span class="font-mono">all</span>
					to share your flow, and <span class="font-mono">user</span> to keep it private.
					<a href="https://docs.windmill.dev/docs/reference/namespaces">docs</a>
				</div>
			</Path>
			<h3 class="text-gray-700 pb-1 border-b">Metadata</h3>

			<label class="block ">
				<span class="text-gray-700">Summary <Required required={false} /></span>
				<textarea
					bind:value={flow.summary}
					class="
					mt-1
					block
					w-full
					rounded-md
					border-gray-300
					shadow-sm
					focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50
					"
					placeholder="A very short summary of the flow displayed when the flow is listed"
					rows="1"
				/>
			</label>
			<label class="block ">
				<span class="text-gray-700"
					>Description<Required required={false} detail="accept markdown formatting" />
					<textarea
						bind:value={flow.description}
						class="
					mt-1
					block
					w-full
					rounded-md
					border-gray-300
					shadow-sm
					focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50
					"
						placeholder="A description to help users understand what this flow does and how to use it."
						rows="3"
					/>
				</span></label
			>

			<div>
				<h3 class="text-gray-700 ">Description rendered</h3>
				<div
					class="prose mt-5 text-xs shadow-inner shadow-blue p-4 overflow-auto"
					style="max-height: 200px;"
				>
					<SvelteMarkdown source={flow.description ?? ''} />
				</div>
			</div>
		</div>
	{:else if step === 2}
		<FlowEditor bind:flow />
	{:else if step === 3}
		<ScriptSchema
			synchronizedHeader={false}
			bind:summary={flow.summary}
			bind:description={flow.description}
			bind:schema={flow.schema}
		/>
	{/if}
</div>

<style>
	/* .wizard-nav {
		@apply w-1/2 sm:w-1/4;
	} */

	.wizard-nav button {
		max-height: 30px;
	}
</style>
