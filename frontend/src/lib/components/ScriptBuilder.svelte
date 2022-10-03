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

	export let script: Script
	export let initialPath: string = ''
	export let template: 'pgsql' | 'script' = 'script'

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
				<h3 class="text-gray-700 border-b">Language</h3>
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
				<h4 class="text-gray-700  border-b">
					Script Kind <Tooltip
						>In most cases, you will want the General Script. <br />
						Trigger are meant to be used as the first module of flows to trigger them based on watching
						new events externally. <br />
						Failure scripts are used to handle unrecoverable errors of flows and for handling errors
						at the workspace level. <br />
						Command scripts are used when the workspace is associated with a slack workspace to be triggered
						on command.</Tooltip
					>
				</h4>

				{#if script.language == 'deno'}
					<div class="max-w-lg">
						<RadioButton
							label="Script Type"
							options={[
								['General Script', Script.kind.SCRIPT],
								['Trigger Script', Script.kind.TRIGGER]
								// ['Failure Handler', Script.kind.FAILURE],
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
				{:else}
					<div class="max-w-lg">
						<RadioButton
							label="Script Type"
							options={[['General Script', Script.kind.SCRIPT]]}
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
					<h4 class="text-gray-700  border-b">
						Script Template <Tooltip
							>A template is a pre-filled script corresponding to a more specialized use-case</Tooltip
						>
					</h4>

					<div class="max-w-md">
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

				<label class="block ">
					<span class="text-gray-700">Summary <Required required={false} /></span>
					<textarea
						bind:value={script.summary}
						class="
					mt-1
					block
					w-full
					rounded-md
					border-gray-300
					shadow-sm
					focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50
					"
						placeholder="A very short summary of the script displayed when the script is listed"
						rows="1"
					/>
				</label>
				<label class="block" for="inp">
					<span class="text-gray-700"
						>Description<Required required={false} detail="accept markdown formatting" />
						<textarea
							id="inp"
							bind:value={script.description}
							class="
					mt-1
					block
					w-full
					rounded-md
					border-gray-300
					shadow-sm
					focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50
					"
							placeholder="A description to help users understand what this script does and how to use it."
							rows="3"
						/>
					</span>
				</label>

				<label class="block">
					<span class="text-gray-700 mr-2"
						>Save as workspace template <Tooltip
							>Enable your teammates to use this script as a template to write new scripts.</Tooltip
						>
					</span>
					<input type="checkbox" bind:checked={script.is_template} />
				</label>

				<div>
					<h3 class="text-gray-700 ">Description rendered</h3>
					<div
						class="prose mt-5 text-xs shadow-inner shadow-blue p-4 overflow-auto"
						style="max-height: 200px;"
					>
						<SvelteMarkdown source={script.description ?? ''} />
					</div>
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
