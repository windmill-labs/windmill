<script lang="ts">
	import { ScriptService, type Script } from '$lib/gen'

	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { inferArgs } from '$lib/infer'
	import {
		DENO_INIT_CODE,
		DENO_INIT_CODE_TRIGGER,
		initialCode,
		POSTGRES_INIT_CODE
	} from '$lib/script_helpers'
	import { workspaceStore } from '$lib/stores'
	import { emptySchema, encodeState, sendUserToast, setQueryWithoutLoad } from '$lib/utils'
	import { Breadcrumb, BreadcrumbItem } from 'flowbite-svelte'
	import { onDestroy } from 'svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import Path from './Path.svelte'
	import RadioButton from './RadioButton.svelte'
	import Required from './Required.svelte'
	import ScriptEditor from './ScriptEditor.svelte'
	import ScriptSchema from './ScriptSchema.svelte'
	import CenteredPage from './CenteredPage.svelte'

	let editor: ScriptEditor
	let scriptSchema: ScriptSchema

	export let script: Script
	export let initialPath: string = ''
	export let template: 'pgsql' | undefined = undefined

	let pathError = ''

	$: setQueryWithoutLoad($page.url, 'state', encodeState(script))
	$: step = Number($page.url.searchParams.get('step')) || 1
	$: {
		if (script.language == 'python3') {
			script.is_trigger = false
		}
	}

	if (script.content == '') {
		initContent(script.language, template)
	}

	function initContent(language: 'deno' | 'python3', template: 'pgsql' | undefined) {
		script.content = initialCode(
			language,
			template == 'pgsql' ? 'pgsql' : script.is_trigger ? 'trigger' : undefined
		)
	}

	async function editScript(): Promise<void> {
		try {
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
					is_trigger: script.is_trigger
				}
			})
			sendUserToast(`Success! New script version created with hash ${newHash}`)
			goto(`/scripts/get/${newHash}`)
		} catch (error) {
			sendUserToast(`Impossible to save the script: ${error.body}`, true)
		}
	}

	export function setCode(script: Script) {
		editor?.getEditor().setCode(script.content)

		if (scriptSchema) {
			if (script.schema) {
				scriptSchema.setSchema(script.schema)
			} else {
				scriptSchema.setSchema(emptySchema())
			}
		}
	}

	async function inferSchema() {
		await inferArgs(script.language, script.content, script.schema)
	}

	async function changeStep(step: number) {
		if (step == 3) {
			script.content = editor?.getEditor().getCode() ?? script.content
			await inferSchema()
			script.schema = script.schema
		}
		goto(`?step=${step}`)
	}

	onDestroy(() => {
		editor?.$destroy()
	})
</script>

<div class="flex flex-col h-screen">
	<!-- Nav between steps-->
	<div class="flex flex-col w-full px-4 py-2 border-b shadow-sm">
		<div class="justify-between flex flex-row drop-shadow-sm w-full">
			<div class="wizard-nav flex flex-row w-full">
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
					<button
						disabled={step == 1 && pathError != ''}
						class="default-button px-6 max-h-8"
						on:click={() => {
							changeStep(step + 1)
						}}
					>
						Next
					</button>
				{:else}
					<button class="default-button px-6 self-end" on:click={editScript}>Save</button>
				{/if}
				{#if step > 1}
					<button
						class="default-button-secondary px-6 max-h-8 mr-2"
						on:click={async () => {
							changeStep(step - 1)
						}}
					>
						Back
					</button>
				{/if}
				{#if step == 2}
					<button
						class="default-button-secondary px-6 max-h-8 mr-2"
						on:click={async () => {
							await inferSchema()
							editScript()
						}}
					>
						Save (commit)
					</button>
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
							['Python 3.10', 'python3']
						]}
						on:change={(e) => initContent(e.detail, template)}
						bind:value={script.language}
					/>
				</div>
				{#if script.language == 'deno'}
					<h4 class="text-gray-700  border-b">Template</h4>

					<div class="max-w-md">
						<RadioButton
							label="Template"
							options={[
								['None', undefined],
								['PostgreSQL', 'pgsql']
							]}
							on:change={(e) => initContent(script.language, e.detail)}
							bind:value={template}
						/>
					</div>
				{/if}

				<h3 class="text-gray-700 pb-1 border-b">Metadata</h3>

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
					<span class="text-gray-700 mr-2">Save as template</span>
					<input type="checkbox" bind:checked={script.is_template} />
				</label>

				<label class="block">
					<span class="text-gray-700 mr-2">Save as trigger script</span>
					<input
						disabled={script.language == 'python3'}
						type="checkbox"
						bind:checked={script.is_trigger}
						on:change={() => {
							if (
								script.content == DENO_INIT_CODE ||
								script.content == DENO_INIT_CODE_TRIGGER ||
								script.content == POSTGRES_INIT_CODE
							) {
								initContent(script.language, template)
							}
						}}
					/>
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
			bind:this={editor}
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

<style>
	/* .wizard-nav {
		@apply w-1/2 sm:w-1/4;
	} */
	.wizard-nav button {
		max-height: 30px;
	}
</style>
