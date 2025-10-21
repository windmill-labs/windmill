<script lang="ts">
	import { ArrowLeft } from 'lucide-svelte'
	import { UserService } from '$lib/gen/services.gen'
	import { goto } from '$lib/navigation'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
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
	} from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'

	let currentStep = $state(1)
	let useCaseText = $state('')
	let selectedSource = $state<string | null>(null)
	let isSubmitting = $state(false)

	const sources = [
		{ id: 'search_engine', label: 'Search engine', icon: Search },
		{ id: 'youtube', label: 'Youtube', icon: Youtube },
		{ id: 'linkedin', label: 'LinkedIn', icon: Linkedin },
		{ id: 'twitter', label: 'X/Twitter', icon: Twitter },
		{ id: 'github', label: 'GitHub', icon: Github },
		{ id: 'blog', label: 'Blog/Article', icon: FileText },
		{ id: 'word_of_mouth', label: 'Word of mouth', icon: Users },
		{ id: 'in_my_company', label: 'Used in my company', icon: Building2 },
		{ id: 'event', label: 'Event', icon: Calendar },
		{ id: 'other', label: 'Other', icon: HelpCircle }
	]

	function selectSource(sourceId: string) {
		selectedSource = sourceId
		// Auto-advance to next step for all selections
		currentStep = 2
	}

	function goToPreviousStep() {
		if (currentStep === 2) {
			currentStep = 1
		}
	}

	async function continueToWorkspaces() {
		if (!selectedSource || isSubmitting) return

		isSubmitting = true
		try {
			// Get email from global user info instead of userStore (which is workspace-specific)
			const globalUserInfo = await UserService.globalWhoami()
			const email = globalUserInfo.email

			if (!email) {
				console.error('No email found in global user info:', globalUserInfo)
				throw new Error('User email not found')
			}

			const touchPoint = selectedSource

			await UserService.submitOnboardingData({
				requestBody: {
					email,
					customer_id: null,
					is_ee_trial: null,
					touch_point: touchPoint,
					use_case: useCaseText
				}
			})

			sendUserToast('Information saved successfully')
		} catch (error) {
			console.error('Error submitting onboarding data:', error)
			sendUserToast('Failed to save information: ' + (error?.body || error?.message || error), true)
		} finally {
			isSubmitting = false
			goto('/user/workspaces')
		}
	}

	function skip() {
		goto('/user/workspaces')
	}
</script>

{#if currentStep === 1}
	<CenteredModal title="Where did you hear about Windmill?">
		<div class="w-full max-w-lg mx-auto">
			<div class="grid grid-cols-1 gap-3 mt-6 mb-6">
				{#each sources as source (source.id)}
					<button
						onclick={() => selectSource(source.id)}
						class="flex items-center gap-3 px-4 py-3 bg-surface-secondary rounded-lg border transition-all hover:bg-surface-hover {selectedSource ===
						source.id
							? 'border-blue-500 ring-2 ring-blue-500 ring-opacity-50'
							: 'border-gray-200 dark:border-gray-700'}"
					>
						<svelte:component this={source.icon} class="w-5 h-5 text-primary" />
						<span class="text-sm font-medium text-primary">{source.label}</span>
					</button>
				{/each}
			</div>

			<div class="flex flex-row justify-end items-center pt-4">
				<Button color="light" variant="border" size="xs" on:click={skip}>Skip</Button>
			</div>

			<div class="flex justify-center mt-4">
				<div class="flex items-center gap-2">
					<div class="w-2 h-2 rounded-full bg-blue-500"></div>
					<div class="w-2 h-2 rounded-full bg-gray-300 dark:bg-gray-600"></div>
				</div>
			</div>
		</div>
	</CenteredModal>
{:else if currentStep === 2}
	<CenteredModal title="What do you want to use Windmill for?">
		<div class="w-full max-w-lg mx-auto">
			<p class="text-sm text-secondary mb-6">
				This will help us provide tailored support for your specific needs.
			</p>
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
				</div>
			</div>
		</div>
	</CenteredModal>
{/if}
