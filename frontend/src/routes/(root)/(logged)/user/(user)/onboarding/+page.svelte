<script lang="ts">
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
		Bot,
		Gauge,
		Database,
		Workflow,
		Webhook
	} from 'lucide-svelte'

	let currentStep = $state(1)
	let selectedUseCase = $state<string | null>(null)
	let selectedSource = $state<string | null>(null)

	const useCases = [
		{ id: 'ai_agents', label: 'AI Agents', icon: Bot },
		{ id: 'document_ingestion', label: 'Document Ingestion', icon: FileText },
		{ id: 'data_pipelines', label: 'Data Pipelines', icon: Database },
		{ id: 'internal_automations', label: 'Internal Automations', icon: Workflow },
		{ id: 'webhooks', label: 'Webhooks', icon: Webhook },
		{ id: 'Dashboards', label: 'Dashboards', icon: Gauge },
		{ id: 'other', label: 'Other', icon: HelpCircle }
	]

	const sources = [
		{ id: 'AI Search', label: 'AI Search', icon: Bot },
		{ id: 'search_engine', label: 'Search Engine', icon: Search },
		{ id: 'linkedin', label: 'LinkedIn', icon: Linkedin },
		{ id: 'twitter', label: 'Twitter', icon: Users },
		{ id: 'blog', label: 'Blog/Article', icon: FileText },
		{ id: 'word_of_mouth', label: 'Word of Mouth', icon: Users },
		{ id: 'github', label: 'GitHub', icon: Github },
		{ id: 'event', label: 'Event', icon: Calendar },
		{ id: 'other', label: 'Other', icon: HelpCircle }
	]

	function selectUseCase(useCaseId: string) {
		selectedUseCase = useCaseId
	}

	function selectSource(sourceId: string) {
		selectedSource = sourceId
	}

	function goToNextStep() {
		if (currentStep === 1 && selectedUseCase) {
			currentStep = 2
		}
	}

	function goToPreviousStep() {
		if (currentStep === 2) {
			currentStep = 1
		}
	}

	async function continueToWorkspaces() {
		if (!selectedSource) return

		// TODO: Send the selected data to the backend API
		// await UserService.updateOnboardingInfo({
		//   useCase: selectedUseCase,
		//   referralSource: selectedSource
		// })

		goto('/user/workspaces')
	}

	function skip() {
		goto('/user/workspaces')
	}
</script>

{#if currentStep === 1}
	<CenteredModal title="What are you building?">
		<p class="text-center text-sm text-secondary mb-6">
			Help us personalize your onboarding experience
		</p>
		<div class="w-full max-w-lg mx-auto">
			<div class="grid grid-cols-1 gap-3 mb-6">
				{#each useCases as useCase (useCase.id)}
					<button
						onclick={() => selectUseCase(useCase.id)}
						class="flex items-center gap-3 px-4 py-3 bg-surface-secondary rounded-lg border transition-all hover:bg-surface-hover {selectedUseCase ===
						useCase.id
							? 'border-blue-500 ring-2 ring-blue-500 ring-opacity-50'
							: 'border-gray-200 dark:border-gray-700'}"
					>
						<svelte:component this={useCase.icon} class="w-5 h-5 text-primary" />
						<span class="text-sm font-medium text-primary">{useCase.label}</span>
					</button>
				{/each}
			</div>

			<div class="flex flex-row justify-between items-center pt-4 gap-4">
				<Button color="light" variant="border" size="xs" on:click={skip}>Skip</Button>
				<Button
					color="blue"
					variant="contained"
					size="lg"
					disabled={!selectedUseCase}
					on:click={goToNextStep}
				>
					Continue
				</Button>
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

			<div class="flex flex-row justify-between items-center pt-4 gap-4">
				<Button color="light" variant="border" size="xs" on:click={goToPreviousStep}>
					Previous
				</Button>
				<Button
					color="blue"
					variant="contained"
					size="lg"
					disabled={!selectedSource}
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
