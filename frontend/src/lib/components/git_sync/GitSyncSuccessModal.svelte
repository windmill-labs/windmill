<script lang="ts">
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { CheckCircle2, ExternalLink, ArrowRight } from 'lucide-svelte'

	interface Props {
		open: boolean
		savedWithoutInit?: boolean
		autoPullOn?: boolean
	}

	let { open = $bindable(false), savedWithoutInit = false, autoPullOn = false }: Props = $props()
</script>

<Modal bind:open title="Git Sync Connection Saved" class="sm:max-w-4xl" cancelText="Close">
	<div class="flex flex-col gap-6 p-6">
		<!-- Success header -->
		<div class="flex items-center gap-3">
			<div class="flex-shrink-0">
				<CheckCircle2 class="h-8 w-8 text-green-600" />
			</div>
			<div>
				<h3 class="text-lg font-semibold text-primary">Git sync connection saved successfully!</h3>
				<p class="text-sm text-secondary mt-1"
					>Your repository is now configured to receive changes from Windmill.</p
				>
			</div>
		</div>

		<!-- Info box for saved without init -->
		{#if savedWithoutInit}
			<div class="bg-blue-50 border border-blue-200 rounded-lg p-4">
				<h4 class="font-medium text-blue-900 mb-2">Repository saved without initialization</h4>
				<p class="text-sm text-blue-800">
					Only new changes will be pushed to this repository. Existing content in Windmill has not
					been initialized to the repository.
				</p>
			</div>
		{/if}

		<!-- Optional setup section -->
		{#if autoPullOn}
			<div class="bg-green-50 border border-green-200 rounded-lg p-4">
				<h4 class="font-medium text-green-900 mb-2 flex items-center gap-2">
					<ArrowRight class="h-4 w-4" />
					Pull from Git is on
				</h4>
				<p class="text-sm text-green-800">
					New commits to the tracked branch deploy into this workspace automatically. You can
					adjust this anytime on the repository card.
				</p>
			</div>
		{:else}
			<div class="bg-amber-50 border border-amber-200 rounded-lg p-4">
				<h4 class="font-medium text-amber-900 mb-2 flex items-center gap-2">
					<ArrowRight class="h-4 w-4" />
					Deploy changes from Git back to Windmill
				</h4>
				<p class="text-sm text-amber-800 mb-3">
					Turn on "Automatically deploy changes from Git" on the repository to have Windmill pull
					new commits into this workspace for you.
				</p>
			<div class="flex flex-col gap-2">
				<p class="text-sm text-amber-700">
					Prefer to control deployment from your own pipeline (tests, custom gating, deploy on PR
					merge)? Set up GitHub Actions or similar CI/CD workflows instead.
				</p>
				<div class="mt-3">
					<a
						href="https://www.windmill.dev/docs/advanced/deploy_gh_gl#github-actions-setup"
						target="_blank"
						class="text-sm text-amber-700 hover:text-amber-900 underline flex items-center gap-1"
					>
						<ExternalLink class="h-3 w-3" />
						Learn more about CI-based deployment
					</a>
					</div>
				</div>
			</div>
		{/if}
	</div>
</Modal>
