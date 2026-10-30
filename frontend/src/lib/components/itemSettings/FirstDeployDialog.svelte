<script lang="ts" module>
	import { GranularAclService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import type { AppAccessMode } from './appAccess'
	import type { ShareKind } from './ShareDialog.svelte'

	export type FirstDeployChoice = {
		summary: string
		path: string
		/** Extra members to add once the item exists, owner → write. */
		members: Record<string, boolean>
		/** Apps only. */
		appMode?: AppAccessMode
		message: string
		/** The user the item runs on behalf of as the dialog leaves it, when the caller passed one
		 *  in: the second step can change it or turn it off, and the deploy asks nothing more. */
		runAs?: { email: string | undefined; permissionedAs: string | undefined }
	}
	/** Add the members chosen at the first deploy, once the item exists at `path`. */
	export async function applyFirstDeployMembers(
		workspace: string,
		kind: ShareKind,
		path: string,
		members: Record<string, boolean>
	): Promise<void> {
		for (const [owner, write] of Object.entries(members)) {
			try {
				await GranularAclService.addGranularAcls({
					workspace,
					path,
					kind,
					requestBody: { owner, write }
				})
			} catch (err) {
				sendUserToast(`Deployed, but could not share it with ${owner}: ${err}`, true)
			}
		}
	}
</script>

<script lang="ts">
	import { Button } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Label from '$lib/components/Label.svelte'
	import Path from '$lib/components/Path.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { isPlaceholderName } from '$lib/components/random_positive_adjetive'
	import {
		useOperatingUser,
		useOperatingWorkspace
	} from '$lib/components/operatingWorkspace.svelte'
	import { useFolderDefaultPermissionedAs } from '$lib/components/useFolderDefaultPermissionedAs.svelte'
	import ItemAccessList from './ItemAccessList.svelte'
	import AppAccessSection from './AppAccessSection.svelte'
	import DeployOnBehalfOfFields from './DeployOnBehalfOfFields.svelte'
	import { deployAsksOnBehalfOf, deployOnBehalfOfQuestion } from './onBehalfOf'
	import { logFirstDeploy } from './sharingTelemetry'

	const operatingWorkspace = useOperatingWorkspace()
	const operatingUser = useOperatingUser()

	let open = $state(false)
	let step = $state<'details' | 'runAs'>('details')
	let kind = $state<ShareKind>('flow')
	let summary = $state('')
	let path: string | undefined = $state()
	let pathError = $state('')
	let dirtyPath = $state(false)
	let members: Record<string, boolean> = $state({})
	let appMode = $state<AppAccessMode>('publisher')
	let message = $state('')
	/** Where the item's run-as user is looked up; unset when the caller asks nothing about it. */
	let runAsWorkspace: string | undefined = $state()
	let runAsEmail: string | undefined = $state()
	let runAsPermissionedAs: string | undefined = $state()
	let pathField: Path | undefined = $state()
	let resolve: ((choice: FirstDeployChoice | undefined) => void) | undefined

	let noun = $derived(kind === 'raw_app' ? 'app' : kind)
	let isApp = $derived(kind === 'app' || kind === 'raw_app')
	let runAsKind = $derived<'flow' | 'script' | 'app'>(kind === 'raw_app' ? 'app' : kind)
	// A new item takes its folder's default user, so the second step follows the path chosen in
	// the first.
	const folderDefault = useFolderDefaultPermissionedAs(
		() => (open && runAsWorkspace !== undefined ? path : undefined),
		() => runAsWorkspace
	)
	let asksRunAs = $derived(
		runAsWorkspace !== undefined &&
			deployAsksOnBehalfOf(runAsKind, {
				user: operatingUser.current,
				email: runAsEmail,
				permissionedAs: runAsPermissionedAs,
				savedEmail: undefined,
				savedPermissionedAs: undefined,
				folderDefault: folderDefault.value
			})
	)
	let question = $derived(
		deployOnBehalfOfQuestion(runAsKind, operatingUser.current, {
			email: runAsEmail,
			permissionedAs: runAsPermissionedAs,
			savedEmail: undefined,
			savedPermissionedAs: undefined,
			isNew: true,
			folderDefault: folderDefault.value
		})
	)

	function slug(s: string): string {
		return s
			.toLowerCase()
			.replace(/[^a-z0-9_]/g, '_')
			.replace(/_+/g, '_')
			.replace(/^_|_$/g, '')
	}

	/**
	 * Ask where a never-deployed item goes and who reaches it, then, where a deploy asks it, who
	 * it runs on behalf of. Resolves to the choice, or to undefined when the dialog is dismissed.
	 * The name follows the summary while it is still the placeholder the editor generated.
	 */
	export function ask(opts: {
		kind: ShareKind
		summary: string
		path: string
		/** The caller picked the path, so it does not follow the summary. */
		pathChosen?: boolean
		appMode?: AppAccessMode
		/** The user the item holds to run on behalf of, which the second step confirms. */
		runAs?: {
			email: string | undefined
			permissionedAs: string | undefined
			targetWorkspace: string
		}
	}): Promise<FirstDeployChoice | undefined> {
		resolve?.(undefined)
		step = 'details'
		kind = opts.kind
		runAsWorkspace = opts.runAs?.targetWorkspace
		runAsEmail = opts.runAs?.email
		runAsPermissionedAs = opts.runAs?.permissionedAs
		summary = opts.summary
		pathError = ''
		// A new item is parked at `draft_<uuid>` or named `<adjective>_<kind>` until someone names it.
		const segments = opts.path.split('/')
		const generated =
			isPlaceholderName(opts.path) || /^draft_[0-9a-f_]+$/.test(segments.at(-1) ?? '')
		dirtyPath = !!opts.pathChosen || !generated
		const name = slug(opts.summary)
		// Named here rather than through Path once it is open: Path parses the path it is handed a
		// tick after mounting, and drops a name set before that.
		path = !dirtyPath && name ? [...segments.slice(0, -1), name].join('/') : opts.path
		members = {}
		appMode = opts.appMode ?? 'publisher'
		message = ''
		open = true
		return new Promise((r) => (resolve = r))
	}

	function finish(choice: FirstDeployChoice | undefined) {
		// Modal writes `open` back on every close, an unchanged `false` included: only an `ask`
		// still waiting is answered.
		const r = resolve
		if (!r) return
		resolve = undefined
		logFirstDeploy(choice ? 'deployed' : 'cancelled')
		open = false
		r(choice)
	}

	async function next() {
		if (!path || pathError) return
		// The folder default follows a path change a fetch later: read it for the path chosen.
		if (runAsWorkspace !== undefined) await folderDefault.refetch()
		if (asksRunAs) step = 'runAs'
		else deploy()
	}

	function deploy() {
		if (!path || pathError) return
		finish({
			summary,
			path,
			members: $state.snapshot(members),
			appMode: isApp ? appMode : undefined,
			message,
			runAs:
				runAsWorkspace !== undefined
					? { email: runAsEmail, permissionedAs: runAsPermissionedAs }
					: undefined
		})
	}
</script>

<!-- Closed by Escape at the first step, the backdrop or the close button: a dismissal. On the second
     step Escape goes back to the first. -->
<Modal
	bind:open={() => open, (v) => (v ? (open = true) : finish(undefined))}
	title="Deploy this {noun}"
	trail={step === 'runAs'
		? [
				{ label: `Deploy this ${noun}`, onclick: () => (step = 'details') },
				{ label: question.title }
			]
		: undefined}
	description={step === 'details'
		? 'First deploy: choose its name and who can reach it. Share changes both later.'
		: undefined}
	kind="X"
	enterConfirms={false}
	fitHeight
	class="sm:max-w-xl"
>
	{#if step === 'runAs'}
		<div class="flex flex-col gap-4">
			<DeployOnBehalfOfFields
				kind={runAsKind}
				bind:email={runAsEmail}
				bind:permissionedAs={runAsPermissionedAs}
				savedEmail={undefined}
				savedPermissionedAs={undefined}
				isNew
				folderDefault={folderDefault.value}
				targetWorkspace={runAsWorkspace ?? ''}
				executionMode={isApp ? appMode : undefined}
				rawApp={kind === 'raw_app'}
			/>
			<div class="flex flex-row justify-end gap-2">
				<Button variant="default" unifiedSize="md" onclick={() => (step = 'details')}>Back</Button>
				<Button variant="accent" unifiedSize="md" disabled={folderDefault.loading} onclick={deploy}
					>Deploy</Button
				>
			</div>
		</div>
	{/if}
	<!-- Kept mounted on the second step, so going back finds the path and the people as they
	     were left. -->
	<div class="flex flex-col gap-6 h-full {step === 'details' ? '' : 'hidden'}">
		<Label label="Summary" for="first-deploy-summary">
			<TextInput
				inputProps={{
					id: 'first-deploy-summary',
					placeholder: 'Short summary to be displayed when listed',
					oninput: (e: Event) => {
						const typed = slug((e.currentTarget as HTMLInputElement).value)
						if (!dirtyPath && typed) pathField?.setName(typed)
					}
				}}
				bind:value={summary}
			/>
		</Label>

		<Label label="Path">
			<Path
				bind:this={pathField}
				bind:path
				bind:error={pathError}
				bind:dirty={dirtyPath}
				initialPath=""
				kind={kind === 'raw_app' ? 'app' : kind}
				namePlaceholder={noun}
				autofocus={false}
			/>
		</Label>

		{#if path}
			{#key path}
				<ItemAccessList {kind} {path} title="Who gets access" bind:pending={members} />
			{/key}
		{/if}

		{#if isApp}
			<section class="flex flex-col gap-2">
				<h3 class="text-sm font-semibold text-emphasis">Who can open the app</h3>
				<AppAccessSection
					workspace={$operatingWorkspace!}
					appPath={path ?? ''}
					rawApp={kind === 'raw_app'}
					mode={appMode}
					onModeChange={(m) => {
						appMode = m
					}}
				/>
			</section>
		{/if}

		<Label label="Deployment message" for="first-deploy-message">
			<TextInput
				inputProps={{ id: 'first-deploy-message', placeholder: 'Optional' }}
				bind:value={message}
			/>
		</Label>

		<div class="flex flex-row justify-end gap-2">
			<Button variant="default" unifiedSize="md" onclick={() => finish(undefined)}>Cancel</Button>
			<!-- Whether a second step asks who it runs on behalf of waits on the folder default. -->
			<Button
				variant="accent"
				unifiedSize="md"
				disabled={!path || !!pathError || folderDefault.loading}
				onclick={next}
			>
				{asksRunAs ? 'Next' : 'Deploy'}
			</Button>
		</div>
	</div>
</Modal>
