<script lang="ts">
	import { ArrowLeft, Loader2 } from 'lucide-svelte'
	import { UserService, WorkspaceService } from '$lib/gen/services.gen'
	import { goto } from '$lib/navigation'
	import { usersWorkspaceStore } from '$lib/stores'
	import { switchWorkspace } from '$lib/storeUtils'
	import { page } from '$app/state'
	import { toSameOriginRelativePath } from '$lib/logoutRedirect'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import CreateWorkspaceInner from '$lib/components/workspaceSettings/CreateWorkspaceInner.svelte'
	import { WORKSPACE_NAME_MAX_LENGTH } from '$lib/utils/workspaceId'
	import { defaultWorkspaceName, WORKSPACE_HANDOVER_MS } from '$lib/workspaceCreation'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import {
		Search,
		Linkedin,
		Users,
		FileText,
		Github,
		Calendar,
		HelpCircle,
		Building2,
		Twitter,
		Youtube,
		Bot,
		MessageCircleCode
	} from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'

	// Define step names as constants for better maintainability
	const STEP_SOURCE = 'source'
	const STEP_USE_CASE = 'use_case'
	const STEP_WORKSPACE = 'workspace'

	type OnboardingStep = typeof STEP_SOURCE | typeof STEP_USE_CASE | typeof STEP_WORKSPACE

	let currentStep = $state<OnboardingStep>(STEP_SOURCE)
	let useCaseText = $state('')
	let selectedSource = $state<string | null>(null)
	let isSubmitting = $state(false)
	let otherSourceText = $state('')
	let otherPopoverOpen = $state(false)
	let otherInputRef: HTMLInputElement | undefined = $state()

	// The workspace signup made for this user, and the name they give it. Loaded up front so
	// the step is ready by the time the survey is answered — and dropped altogether for someone
	// who arrived by invite, who has a workspace already and was never given one of their own.
	let ownWorkspace = $state<{ id: string; name: string } | undefined>(undefined)
	let workspaceName = $state('')
	let renaming = $state(false)
	// The survey was skipped, so the naming step has nothing to go back to.
	let skippedSurvey = $state(false)
	// Everything a name does not cover — id, colour, username, invites — behind the real
	// creation form rather than a second copy of it here. It makes a workspace of its own,
	// which is the point: this is the escape hatch for someone setting up for a team.
	let advanced = $state(false)

	async function loadWorkspaceStep() {
		try {
			const [me, workspaces] = await Promise.all([
				UserService.globalWhoami(),
				WorkspaceService.listUserWorkspaces()
			])
			usersWorkspaceStore.set(workspaces)
			const owned = workspaces.workspaces.filter((w) => w.id !== 'admins')
			if (owned.length !== 1) return
			ownWorkspace = { id: owned[0].id, name: owned[0].name }
			workspaceName = defaultWorkspaceName(me.name, me.email)
		} catch (error) {
			console.error('Could not prepare the workspace step:', error)
		}
	}
	void loadWorkspaceStep()

	const sources = [
		{ id: 'ai_search', label: 'AI search', icon: Bot },
		{ id: 'search_engine', label: 'Search engine', icon: Search },
		{ id: 'reddit', label: 'Reddit', icon: MessageCircleCode },
		{ id: 'youtube', label: 'Youtube', icon: Youtube },
		{ id: 'github', label: 'GitHub', icon: Github },
		{ id: 'in_my_company', label: 'Current/Previous company', icon: Building2 },
		{ id: 'word_of_mouth', label: 'Word of mouth', icon: Users },
		{ id: 'blog', label: 'Blog/Article', icon: FileText },
		{ id: 'linkedin', label: 'LinkedIn', icon: Linkedin },
		{ id: 'twitter', label: 'X/Twitter', icon: Twitter },
		{ id: 'event', label: 'Event', icon: Calendar },
		{ id: 'other', label: 'Other', icon: HelpCircle }
	]

	// Focus the "Other" input when the popover opens
	$effect(() => {
		if (otherPopoverOpen && otherInputRef) {
			otherInputRef.focus()
		}
	})

	function selectSource(sourceId: string) {
		selectedSource = sourceId
		// Auto-advance to next step
		currentStep = STEP_USE_CASE
	}

	function validateOtherSource() {
		if (otherSourceText.trim()) {
			selectedSource = `other: ${otherSourceText.trim()}`
		} else {
			selectedSource = 'other'
		}
		otherPopoverOpen = false
		// Auto-advance to next step
		currentStep = STEP_USE_CASE
	}

	function goToPreviousStep() {
		currentStep = currentStep === STEP_WORKSPACE ? STEP_USE_CASE : STEP_SOURCE
	}

	const workspaceNameProblem = $derived(
		!workspaceName.trim()
			? 'A name is required'
			: workspaceName.trim().length > WORKSPACE_NAME_MAX_LENGTH
				? `The name is too long (max ${WORKSPACE_NAME_MAX_LENGTH} characters).`
				: undefined
	)

	/** Renames the workspace signup created, then leaves. A failure is not worth blocking on:
	 *  the workspace already carries the name the backend derived, which is the same one
	 *  prefilled here, so the user loses an edit rather than a workspace. */
	async function confirmWorkspaceName() {
		if (!ownWorkspace || workspaceNameProblem || renaming) return
		renaming = true
		const next = workspaceName.trim()
		const started = Date.now()
		try {
			if (next !== ownWorkspace.name) {
				await WorkspaceService.changeWorkspaceName({
					workspace: ownWorkspace.id,
					requestBody: { new_name: next }
				})
			}
		} catch (error) {
			console.error('Could not rename the workspace:', error)
			sendUserToast('Could not rename the workspace: ' + (error?.body || error?.message), true)
		} finally {
			const left = WORKSPACE_HANDOVER_MS - (Date.now() - started)
			if (left > 0) await new Promise((resolve) => setTimeout(resolve, left))
			// `renaming` is left up: it is what draws the hand-over, and the navigation below
			// loads the workspace for the first time — clearing it would show the form again
			// underneath for as long as that takes.
			leaveOnboarding()
		}
	}

	/**
	 * Where to go once onboarding is done. A destination carried by the sign-in — a hub
	 * project import, say — is what the user came for, so it wins. Otherwise cloud signup
	 * has made one workspace for them and the picker would be a page with a single choice on
	 * it: land in that workspace, and fall back to the picker only when there is an actual
	 * choice to make (an invite to accept, several workspaces, none).
	 */
	async function leaveOnboarding() {
		// `toSameOriginRelativePath` rather than a local check: it already rejects `//host`,
		// `/\\host` (which WHATWG URL parsing resolves to a different origin), control
		// characters and oversized values. A second, weaker copy of this is how one of those
		// gets missed.
		const requested = toSameOriginRelativePath(page.url.searchParams.get('rd'))
		if (requested) {
			await goto(requested)
			return
		}
		try {
			const workspaces = await WorkspaceService.listUserWorkspaces()
			usersWorkspaceStore.set(workspaces)
			const owned = workspaces.workspaces.filter((w) => w.id !== 'admins')
			if (owned.length === 1) {
				switchWorkspace(owned[0].id)
				await goto('/')
				return
			}
		} catch (error) {
			console.error('Could not list workspaces after onboarding:', error)
		}
		await goto('/user/workspaces')
	}

	async function continueToWorkspaces() {
		if (!selectedSource || isSubmitting) return

		isSubmitting = true
		try {
			await UserService.submitOnboardingData({
				requestBody: {
					touch_point: selectedSource,
					use_case: useCaseText
				}
			})

			sendUserToast('Information saved successfully')
		} catch (error) {
			console.error('Error submitting onboarding data:', error)
			sendUserToast('Failed to save information: ' + (error?.body || error?.message || error), true)
		} finally {
			isSubmitting = false
			// do not block users from accessing windmill even if there is an error
			if (ownWorkspace) {
				currentStep = STEP_WORKSPACE
			} else {
				leaveOnboarding()
			}
		}
	}

	async function skip() {
		isSubmitting = true
		try {
			await UserService.submitOnboardingData({
				requestBody: {}
			})
		} catch (error) {
			console.error('Error skipping onboarding:', error)
		} finally {
			isSubmitting = false
			// Skipping the survey is not skipping naming the workspace: the questions are ours,
			// the workspace is theirs.
			skippedSurvey = true
			if (ownWorkspace) {
				currentStep = STEP_WORKSPACE
			} else {
				leaveOnboarding()
			}
		}
	}
</script>

{#if currentStep === STEP_SOURCE}
	<CenteredModal title="How did you hear about Windmill?">
		<div class="w-full max-w-lg mx-auto">
			<div class="grid grid-cols-1 gap-2 mt-6 mb-6">
				{#each sources as source (source.id)}
					{#if source.id === 'other'}
						<Popover bind:isOpen={otherPopoverOpen} placement="bottom" contentClasses="p-4 w-96">
							{#snippet trigger()}
								<Button
									variant="default"
									unifiedSize="md"
									selected={selectedSource === 'other' || selectedSource?.startsWith('other:')}
									startIcon={{ icon: source.icon }}
									btnClasses="!justify-start w-full"
								>
									{source.label}
								</Button>
							{/snippet}
							{#snippet content()}
								<div class="flex flex-col gap-3">
									<input
										type="text"
										bind:this={otherInputRef}
										bind:value={otherSourceText}
										placeholder="Type your answer..."
										class="input"
									/>
									<Button variant="accent" unifiedSize="md" on:click={validateOtherSource}>
										Validate
									</Button>
								</div>
							{/snippet}
						</Popover>
					{:else}
						<Button
							variant="default"
							unifiedSize="md"
							selected={selectedSource === source.id}
							startIcon={{ icon: source.icon }}
							btnClasses="!justify-start"
							on:click={() => selectSource(source.id)}
						>
							{source.label}
						</Button>
					{/if}
				{/each}
			</div>

			<div class="flex flex-row justify-end items-center pt-4">
				<Button color="light" variant="border" size="xs" on:click={skip} loading={isSubmitting}
					>Skip</Button
				>
			</div>

			<div class="flex justify-center mt-4">
				<div class="flex items-center gap-2">
					<div class="w-2 h-2 rounded-full bg-blue-500"></div>
					<div class="w-2 h-2 rounded-full bg-gray-300 dark:bg-gray-600"></div>
				</div>
			</div>
		</div>
	</CenteredModal>
{:else if currentStep === STEP_USE_CASE}
	<CenteredModal title="What is your primary use case for Windmill?">
		<div class="w-full max-w-lg mx-auto">
			<div class="mb-6">
				<textarea
					bind:value={useCaseText}
					placeholder="E.g., Building AI agents, automating data pipelines, creating internal tools..."
					class="input mt-1"
					rows="8"
					maxlength="1000"
				></textarea>
			</div>

			<div class="flex flex-row justify-between items-center pt-4 gap-4">
				<Button
					color="light"
					variant="border"
					startIcon={{ icon: ArrowLeft }}
					size="xs"
					on:click={goToPreviousStep}
				>
					Previous
				</Button>
				<Button
					color="blue"
					variant="contained"
					size="lg"
					disabled={isSubmitting}
					loading={isSubmitting}
					on:click={continueToWorkspaces}
				>
					Continue
				</Button>
			</div>

			<div class="flex justify-center mt-4">
				<div class="flex items-center gap-2">
					<div class="w-2 h-2 rounded-full bg-gray-300 dark:bg-gray-600"></div>
					<div class="w-2 h-2 rounded-full bg-blue-500"></div>
					{#if ownWorkspace}
						<div class="w-2 h-2 rounded-full bg-gray-300 dark:bg-gray-600"></div>
					{/if}
				</div>
			</div>
		</div>
	</CenteredModal>
{:else if currentStep === STEP_WORKSPACE}
	<CenteredModal title="Name your workspace" centerVertically={false}>
		<div class="w-full max-w-lg mx-auto">
			{#if renaming}
				<div class="flex flex-col items-center gap-3 py-12 text-sm text-secondary">
					<Loader2 size={20} class="animate-spin" />
					Setting up {workspaceName.trim()}…
				</div>
			{:else if advanced}
				<!-- The real creation form: id, colour, username, invites. It makes a workspace of
				     its own and enters it, so this page only has to say where to go afterwards —
				     `goto` rather than `leaveOnboarding`, which would see two workspaces and offer
				     the picker for a choice the user has just made. -->
				<CreateWorkspaceInner inModal onFinish={() => goto('/')} />
			{:else}
				<p class="text-sm text-secondary">
					Your scripts, flows and apps live here. You can rename it later in the workspace settings.
				</p>

				<div class="mt-6 mb-2">
					<TextInput
						bind:value={workspaceName}
						inputProps={{ autofocus: true, maxlength: WORKSPACE_NAME_MAX_LENGTH }}
					/>
					{#if workspaceNameProblem && workspaceName.trim()}
						<span class="text-2xs font-normal text-red-500">{workspaceNameProblem}</span>
					{/if}
				</div>

				<button
					class="text-xs text-secondary hover:text-emphasis"
					onclick={() => (advanced = true)}
				>
					Advanced settings
				</button>

				<div class="flex flex-row justify-between items-center pt-6 gap-4">
					{#if skippedSurvey}
						<span></span>
					{:else}
						<Button
							variant="default"
							unifiedSize="xs"
							startIcon={{ icon: ArrowLeft }}
							on:click={goToPreviousStep}
						>
							Previous
						</Button>
					{/if}
					<Button
						variant="accent"
						unifiedSize="md"
						disabled={!!workspaceNameProblem}
						on:click={confirmWorkspaceName}
					>
						Continue
					</Button>
				</div>
			{/if}

			<div class="flex justify-center mt-4">
				<div class="flex items-center gap-2">
					<div class="w-2 h-2 rounded-full bg-gray-300 dark:bg-gray-600"></div>
					<div class="w-2 h-2 rounded-full bg-gray-300 dark:bg-gray-600"></div>
					<div class="w-2 h-2 rounded-full bg-blue-500"></div>
				</div>
			</div>
		</div>
	</CenteredModal>
{/if}
