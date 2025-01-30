<script lang="ts">
	import EmptyInlineScript from '../apps/editor/inlineScriptsPanel/EmptyInlineScript.svelte'
	import InlineScriptRunnableByPath from '../apps/editor/inlineScriptsPanel/InlineScriptRunnableByPath.svelte'
	import type { RunnableWithFields, StaticAppInput } from '../apps/inputType'
	import { createEventDispatcher } from 'svelte'
	import RawAppInlineScriptEditor from './RawAppInlineScriptEditor.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Tabs from '../common/tabs/Tabs.svelte'
	import { Tab } from '../common'
	import RawAppInputsSpecEditor from './RawAppInputsSpecEditor.svelte'
	import SplitPanesWrapper from '../splitPanes/SplitPanesWrapper.svelte'
	import SchemaForm from '../SchemaForm.svelte'
	import RunnableJobPanelInner from '../apps/editor/RunnableJobPanelInner.svelte'
	import TestJobLoader from '../TestJobLoader.svelte'
	import type { Job } from '$lib/gen'

	export let runnable: RunnableWithFields | undefined
	export let id: string
	export let appPath: string

	const dispatch = createEventDispatcher()

	async function fork(nrunnable: RunnableWithFields) {
		runnable = nrunnable == undefined ? undefined : { ...runnable, ...nrunnable }
	}

	function onPick(o: { runnable: RunnableWithFields; fields: Record<string, StaticAppInput> }) {
		runnable =
			o.runnable == undefined
				? undefined
				: {
						...(runnable ?? {}),
						...o.runnable,
						fields: o.fields
				  }
	}

	let selectedTab = 'inputs'
	let args = {}

	function getSchema(runnable: RunnableWithFields) {
		if (runnable?.type == 'runnableByPath') {
			return {}
		} else if (runnable?.type == 'runnableByName' && runnable.inlineScript) {
			return runnable.inlineScript.schema
		}
		return {}
	}

	let testJobLoader: TestJobLoader | undefined
	let testJob: Job | undefined
	let testIsLoading = false
	let scriptProgress = 0

	// export async function runTest(args: any) {

	// 	// let jobId: string | undefined = undefined
	// 	if (val.type == 'rawscript') {
	// 		await testJobLoader?.runPreview(
	// 			val.path ?? ($pathStore ?? '') + '/' + mod.id,
	// 			val.content,
	// 			val.language,
	// 			args,
	// 		)
	// 	} else if (val.type == 'script') {
	// 		// const script = val.hash
	// 		// 	? await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash: val.hash })
	// 		// 	: await getScriptByPath(val.path)
	// 		// await testJobLoader?.runPreview(
	// 		// 	val.path,
	// 		// 	script.content,
	// 		// 	script.language,
	// 		// 	args,
	// 		// 	$flowStore?.tag ?? (val.tag_override ? val.tag_override : script.tag)
	// 		)
	// 	} else if (val.type == 'flow') {
	// 		// await testJobLoader?.runFlowByPath(val.path, args)
	// 	} else {
	// 		throw Error('Not supported module type')
	// 	}
	// }
</script>

<TestJobLoader
	bind:scriptProgress
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>
{#if runnable?.type == 'runnableByPath' || (runnable?.type == 'runnableByName' && runnable.inlineScript)}
	<Splitpanes>
		<Pane size={55}>
			{#if runnable?.type === 'runnableByName' && runnable.inlineScript}
				{#if runnable.inlineScript.language == 'frontend'}
					<div class="text-sm text-tertiary">Frontend scripts not supported for raw apps</div>
				{:else}
					<RawAppInlineScriptEditor
						on:createScriptFromInlineScript={() =>
							dispatch('createScriptFromInlineScript', runnable)}
						{id}
						bind:inlineScript={runnable.inlineScript}
						bind:name={runnable.name}
						bind:fields={runnable.fields}
						isLoading={testIsLoading}
						onRun={async () => {
							selectedTab = 'test'
							if (
								runnable?.type == 'runnableByName' &&
								runnable.inlineScript?.language != 'frontend'
							) {
								await testJobLoader?.runPreview(
									appPath + '/' + id,
									runnable.inlineScript?.content ?? '',
									runnable.inlineScript?.language,
									args,
									undefined
								)
							}
						}}
						onCancel={async () => {
							if (testJobLoader) {
								await testJobLoader.cancelJob()
							}
						}}
						on:delete
						path={appPath}
					/>
				{/if}
			{:else if runnable?.type == 'runnableByPath'}
				<InlineScriptRunnableByPath
					rawApps
					bind:runnable
					bind:fields={runnable.fields}
					on:fork={(e) => fork(e.detail)}
					on:delete
					{id}
					isLoading={testIsLoading}
					onRun={async () => {
						selectedTab = 'test'
						if (testJobLoader && runnable?.type == 'runnableByPath') {
							if (runnable.runType == 'flow') {
								await testJobLoader.runFlowByPath(runnable.path, args)
							} else if (runnable.runType == 'script' || runnable.runType == 'hubscript') {
								await testJobLoader.runScriptByPath(runnable.path, args)
							}
						}
					}}
					onCancel={async () => {
						if (testJobLoader) {
							await testJobLoader.cancelJob()
						}
					}}
				/>
			{/if}
		</Pane>
		<Pane size={45}>
			<Tabs bind:selected={selectedTab}>
				<Tab value="inputs">Inputs</Tab>
				<Tab value="test">Test</Tab>
				<svelte:fragment slot="content">
					{#if selectedTab == 'inputs'}
						{#if runnable?.fields}
							<div class="w-full flex flex-col gap-4 p-2">
								{#each Object.keys(runnable.fields) as k}
									{@const meta = runnable.fields[k]}
									<RawAppInputsSpecEditor
										key={k}
										bind:componentInput={runnable.fields[k]}
										{id}
										shouldCapitalize
										fieldType={meta?.['fieldType']}
										subFieldType={meta?.['subFieldType']}
										format={meta?.['format']}
										selectOptions={meta?.['selectOptions']}
										tooltip={meta?.['tooltip']}
										placeholder={meta?.['placeholder']}
										customTitle={meta?.['customTitle']}
										loading={meta?.['loading']}
										documentationLink={meta?.['documentationLink']}
										markdownTooltip={meta?.['markdownTooltip']}
										allowTypeChange={meta?.['allowTypeChange']}
										displayType
									/>
								{/each}
							</div>
						{:else}
							<div class="text-tertiary text-sm">No inputs</div>
						{/if}
					{:else if selectedTab == 'test'}
						<SplitPanesWrapper>
							<Splitpanes class="grow">
								<Pane size={50}>
									<div class="px-2 h-full overflow-auto">
										<SchemaForm schema={getSchema(runnable)} bind:value={args} shouldCapitalize />
									</div>
								</Pane>
								<Pane size={50}>
									<RunnableJobPanelInner frontendJob={false} {testJob} {testIsLoading} />
								</Pane>
							</Splitpanes>
						</SplitPanesWrapper>
					{/if}
				</svelte:fragment>
			</Tabs>
		</Pane>
	</Splitpanes>
{:else}
	<EmptyInlineScript
		unusedInlineScripts={[]}
		rawApps
		on:pick={(e) => onPick(e.detail)}
		on:delete
		showScriptPicker
		on:new={(e) => {
			runnable = {
				type: 'runnableByName',
				inlineScript: e.detail,
				name: runnable?.name ?? 'Background Runnable'
			}
		}}
	/>
{/if}
