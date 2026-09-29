import {
	createGitHubAppState,
	loadGithubInstallations,
	startInstallationCheck,
	stopInstallationCheck,
	type GitHubAppState
} from '$lib/githubApp'

/**
 * GitHub App installations for the setup dialog. Owned by the dialog rather than the
 * connect form so the installations are known before the user leaves step 1: the
 * install page has to be opened synchronously from that click, or the browser blocks
 * the new tab.
 */
export class GithubAppSetup {
	gh: GitHubAppState = $state(createGitHubAppState())
	loaded = $state(false)

	constructor(private workspace: string) {}

	get usable() {
		return this.gh.workspaceGithubInstallations.filter((i) => !i.error)
	}

	async reload(): Promise<void> {
		try {
			await loadGithubInstallations(this.gh, this.workspace)
		} catch {
			// loadGithubInstallations toasts its own failure
		}
		this.loaded = true
		const usable = this.usable
		if (!usable.some((i) => i.installation_id === this.gh.selectedGHAppInstallationId)) {
			this.gh.selectedGHAppInstallationId = usable[0]?.installation_id
			this.gh.selectedGHAppRepository = undefined
		}
	}

	/** Must be called from a click handler, synchronously. */
	openInstall(): void {
		const url = this.gh.githubInstallationUrl
		if (!url) return
		window.open(url, '_blank', 'noopener')
		startInstallationCheck(this.gh, this.workspace, () => void this.reload())
	}

	dispose(): void {
		stopInstallationCheck(this.gh)
	}
}
