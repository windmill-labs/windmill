<script lang="ts">
	import { ArrowLeft } from 'lucide-svelte'
	import { UserService, WorkspaceService } from '$lib/gen/services.gen'
	import { goto } from '$lib/navigation'
	import { usersWorkspaceStore } from '$lib/stores'
	import { switchWorkspace } from '$lib/storeUtils'
	import { page } from '$app/state'
	import { toSameOriginRelativePath } from '$lib/logoutRedirect'
	import SimpleCreateWorkspace from '$lib/components/workspaceSettings/SimpleCreateWorkspace.svelte'
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

	// Whether this user has somewhere to go already, in which case there is nothing to create.
	// A pending invite counts: it is a `workspace_invite` row until `accept_invite` runs, so an
	// invited teammate reaches onboarding owning nothing, and creating them a personal
	// workspace is not what they came for — the picker is where the invite is. Loaded up front
	// so the last step is settled by the time the survey is answered, and true when the load
	// fails, since the picker can work the decision out and the create step has no way back.
	let alreadyPlaced = $state(false)
	// The survey was skipped, so the last step has nothing to go back to.
	let skippedSurvey = $state(false)
	// Set by the create form while it hands over to the new workspace.
	let creatingWorkspace = $state(false)

	async function loadWorkspaceStep() {
		try {
			const [workspaces, invites] = await Promise.all([
				WorkspaceService.listUserWorkspaces(),
				UserService.listWorkspaceInvites()
			])
			usersWorkspaceStore.set(workspaces)
			alreadyPlaced = workspaces.workspaces.some((w) => w.id !== 'admins') || invites.length > 0
		} catch (error) {
			console.error('Could not prepare the workspace step:', error)
			alreadyPlaced = true
		}
	}
	// Held, not dropped: Skip awaits one POST that can finish before these GETs do, and
	// branching on `alreadyPlaced` before they land would skip the step this flow exists for.
	// Both exits await it; `isSubmitting` already covers the wait.
	const workspaceStepReady = loadWorkspaceStep()

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

	/**
	 * Where to go once onboarding is done. A destination carried by the sign-in — a hub project
	 * import, say — is what the user came for, so it wins. Otherwise the one workspace this
	 * flow just made, or the one an invite already gave them, is where they belong and the
	 * picker would be a page with a single choice on it. It is reached only when there is an
	 * actual choice to make: several workspaces, or an invite still to accept.
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
			await workspaceStepReady
			isSubmitting = false
			// do not block users from accessing windmill even if there is an error
			if (alreadyPlaced) {
				leaveOnboarding()
			} else {
				currentStep = STEP_WORKSPACE
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
			await workspaceStepReady
			isSubmitting = false
			// Skipping the survey is not skipping naming the workspace: the questions are ours,
			// the workspace is theirs.
			skippedSurvey = true
			if (alreadyPlaced) {
				leaveOnboarding()
			} else {
				currentStep = STEP_WORKSPACE
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
					{#if !alreadyPlaced}
						<div class="w-2 h-2 rounded-full bg-gray-300 dark:bg-gray-600"></div>
					{/if}
				</div>
			</div>
		</div>
	</CenteredModal>
{:else if currentStep === STEP_WORKSPACE}
	<CenteredModal title="Create your workspace" centerVertically={false}>
		<div class="w-full max-w-lg mx-auto">
			<p class="mb-6 text-sm text-secondary">
				Your scripts, flows and apps live here. You can rename it later in the workspace settings.
			</p>

			<!-- The same one-field form the workspace picker falls back to, so a user who leaves
			     onboarding early meets it again rather than something new. It owns the name, the
			     id, the advanced form and the hand-over into the workspace. -->
			<SimpleCreateWorkspace
				onCreated={leaveOnboarding}
				onCreatingChange={(v) => (creatingWorkspace = v)}
			/>

			{#if !skippedSurvey && !creatingWorkspace}
				<div class="flex flex-row justify-start items-center pt-6">
					<Button
						variant="default"
						unifiedSize="xs"
						startIcon={{ icon: ArrowLeft }}
						on:click={goToPreviousStep}
					>
						Previous
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
