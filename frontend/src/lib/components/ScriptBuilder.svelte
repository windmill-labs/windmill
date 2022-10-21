<script lang="ts">
	import { Script, ScriptService } from '$lib/gen'

	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { inferArgs } from '$lib/infer'
	import { initialCode, isInitialCode } from '$lib/script_helpers'
	import { workspaceStore } from '$lib/stores'
	import { encodeState, sendUserToast, setQueryWithoutLoad } from '$lib/utils'
	import { Breadcrumb, BreadcrumbItem } from 'flowbite-svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import Path from './Path.svelte'
	import RadioButton from './RadioButton.svelte'
	import Required from './Required.svelte'
	import ScriptEditor from './ScriptEditor.svelte'
	import ScriptSchema from './ScriptSchema.svelte'
	import CenteredPage from './CenteredPage.svelte'
	import Tooltip from './Tooltip.svelte'
	import UnsavedConfirmationModal from './common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { dirtyStore } from './common/confirmationModal/dirtyStore'
	import { Button } from './common'
	import { slide } from 'svelte/transition'
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'

	export let script: Script
	export let initialPath: string = ''
	export let template: 'pgsql' | 'script' = 'script'

	let viewScriptKind = script.kind !== Script.kind.SCRIPT
	let viewTemplate = script.kind !== Script.kind.SCRIPT && script.language == Script.language.DENO

	let pathError = ''

	$: setQueryWithoutLoad($page.url, 'state', encodeState(script))
	$: step = Number($page.url.searchParams.get('step')) || 1

	if (script.content == '') {
		initContent(script.language, script.kind, template)
	}

	function initContent(
		language: 'deno' | 'python3' | 'go',
		kind: Script.kind,
		template: 'pgsql' | 'script'
	) {
		script.content = initialCode(language, kind, template)
	}

	async function editScript(): Promise<void> {
		try {
			$dirtyStore = false
			if (!script.schema) {
				await inferArgs(script.language, script.content, script.schema)
			}
			const newHash = await ScriptService.createScript({
				workspace: $workspaceStore!,
				requestBody: {
					path: script.path,
					summary: script.summary,
					description: script.description ?? '',
					content: script.content,
					parent_hash: script.hash != '' ? script.hash : undefined,
					schema: script.schema,
					is_template: script.is_template,
					language: script.language,
					kind: script.kind
				}
			})
			sendUserToast(`Success! New script version created with hash ${newHash}`)
			goto(`/scripts/get/${newHash}`)
		} catch (error) {
			sendUserToast(`Impossible to save the script: ${error.body}`, true)
		}
	}

	async function changeStep(step: number) {
		if (step > 1) {
			await inferArgs(script.language, script.content, script.schema)
		}
		goto(`?step=${step}`)
	}
</script>

<UnsavedConfirmationModal />
<div class="flex flex-col h-screen">
	<!-- Nav between steps-->
	<div class="flex flex-col w-full px-4 py-2 border-b shadow-sm">
		<div class="justify-between flex flex-row drop-shadow-sm w-full">
			<div class="flex flex-row w-full">
				<Breadcrumb>
					<BreadcrumbItem>
						<button on:click={() => changeStep(1)} class={step === 1 ? 'font-bold' : null}>
							Metadata
						</button>
					</BreadcrumbItem>
					<BreadcrumbItem>
						<button
							on:click={() => changeStep(2)}
							class={step === 2 ? 'font-bold' : null}
							disabled={pathError != ''}
						>
							Code
						</button>
					</BreadcrumbItem>
					<BreadcrumbItem>
						<button
							on:click={() => changeStep(3)}
							class={step === 3 ? 'font-bold' : null}
							disabled={pathError != ''}
						>
							UI customisation
						</button>
					</BreadcrumbItem>
				</Breadcrumb>
			</div>
			<div class="flex flex-row-reverse ml-2">
				{#if step != 3}
					<Button
						size="sm"
						disabled={step === 1 && pathError !== ''}
						on:click={() => changeStep(step + 1)}
					>
						Next
					</Button>
				{:else}
					<Button size="sm" on:click={editScript}>Save</Button>
				{/if}
				{#if step > 1}
					<Button
						variant="border"
						size="sm"
						btnClasses="mr-2"
						on:click={() => changeStep(step - 1)}
					>
						Back
					</Button>
				{/if}
				{#if step == 2}
					<Button
						variant="border"
						size="sm"
						btnClasses="mr-2"
						on:click={async () => {
							editScript()
						}}
					>
						Save (commit)
					</Button>
				{/if}
			</div>
		</div>
	</div>

	<!-- metadata -->
	{#if step === 1}
		<CenteredPage>
			<div class="space-y-6">
				<h1 class="mb-4">New script</h1>
				<h2 class="border-b pb-1 mt-4">Path & Summary</h2>
				<Path
					bind:error={pathError}
					bind:path={script.path}
					{initialPath}
					on:enter={() => changeStep(2)}
					namePlaceholder="my_script"
					kind="script"
				>
					<div slot="ownerToolkit">
						Script permissions depend on their path. Select the group
						<span class="font-mono"> all </span>
						to share your script, and <span class="font-mono">user</span> to keep it private.
						<a href="https://docs.windmill.dev/docs/reference/namespaces">docs</a>
					</div>
				</Path>
				<label class="block ">
					<span class="text-gray-700">Summary <Required required={false} /></span>
					<textarea
						bind:value={script.summary}
						class="block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 
						focus:ring focus:ring-indigo-200 focus:ring-opacity-50 text-lg"
						placeholder="A very short summary of the script displayed when the script is listed"
						rows="1"
					/>
				</label>
				<h2 class="border-b pb-1 mt-4">Language</h2>
				<div class="max-w-md">
					<RadioButton
						label="Language"
						options={[
							['Typescript (Deno)', 'deno'],
							['Python 3.10', 'python3'],
							['Go', 'go']
						]}
						on:change={(e) => initContent(e.detail, script.kind, template)}
						bind:value={script.language}
					/>
				</div>
				<h2 class="border-b pb-1 mt-4"> Metadata </h2>
				<Button
					color="light"
					size="sm"
					endIcon={{ icon: viewScriptKind ? faChevronUp : faChevronDown }}
					on:click={() => (viewScriptKind = !viewScriptKind)}
				>
					Specialize the script as a specific module kind for flows
				</Button>
				{#if viewScriptKind}
					<div class="max-w-lg" transition:slide>
						<RadioButton
							label="Script Type"
							options={[
								['Common Script', Script.kind.SCRIPT],
								[
									{
										title: 'Trigger Script',
										desc: `First module of flows to trigger them based on watching changes external periodically using an internal state`
									},
									Script.kind.TRIGGER
								],
								[
									{
										title: 'Error Handler',
										desc: `Handle errors for flows after all retries attempts have been exhausted`
									},
									Script.kind.FAILURE
								],
								[
									{
										title: 'Approval Script',
										desc: `Send notification externally to ask for approval to continue a flow`
									},
									Script.kind.APPROVAL
								]

								// ['Command Handler', Script.kind.COMMAND]
							]}
							on:change={(e) => {
								if (isInitialCode(script.content)) {
									template = 'script'
									initContent(script.language, e.detail, template)
								}
							}}
							bind:value={script.kind}
						/>
					</div>
				{/if}
				{#if script.language == 'deno' && script.kind == Script.kind.SCRIPT}
					<Button
						color="light"
						size="sm"
						endIcon={{ icon: viewTemplate ? faChevronUp : faChevronDown }}
						on:click={() => (viewTemplate = !viewTemplate)}
					>
						Use a predefined template specific to this language and script kind
					</Button>

					{#if viewTemplate}
						<div class="max-w-lg" transition:slide>
							<RadioButton
								label="Template"
								options={[
									['Standard', 'script'],
									['PostgreSQL Prepared Statement', 'pgsql']
								]}
								on:change={(e) => initContent(script.language, script.kind, e.detail)}
								bind:value={template}
							/>
						</div>
					{/if}
				{/if}

				<label class="block">
					<span class="text-gray-700 mr-2"
						>Save as workspace template <Tooltip
							>Enable your teammates to use this script as a template to write new scripts.</Tooltip
						>
					</span>
					<input type="checkbox" bind:checked={script.is_template} />
				</label>

				<label class="block" for="inp">
					<span class="text-gray-700 text-sm">
						Description<Required required={false} detail="markdown" />
						<textarea
							id="inp"
							bind:value={script.description}
							class="block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 
							focus:ring focus:ring-indigo-200 focus:ring-opacity-50"
							placeholder="A description to help users understand what this script does and how to use it."
							rows="3"
						/>
					</span>
				</label>

				<div>
					<div class="font-bold pb-1 mt-4">Description preview</div>
					{#if script.description}
						<div class="prose max-h-48 mt-5 text-xs shadow-inner shadow-blue p-4 overflow-auto">
							<SvelteMarkdown source={script.description} />
						</div>
					{:else}
						<div class="text-sm text-gray-500"> Enter a description to see the preview </div>
					{/if}
				</div>
			</div>
		</CenteredPage>
	{:else if step === 2}
		<ScriptEditor
			bind:schema={script.schema}
			path={script.path}
			bind:code={script.content}
			lang={script.language}
		/>
	{:else if step === 3}
		<CenteredPage>
			<ScriptSchema
				bind:summary={script.summary}
				bind:description={script.description}
				bind:schema={script.schema}
			/>
		</CenteredPage>
	{/if}
</div>
