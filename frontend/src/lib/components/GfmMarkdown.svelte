<script lang="ts">
	import { Markdown, type Plugin } from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import rehypeRaw from 'rehype-raw'
	import { rehypeGithubAlerts } from 'rehype-github-alerts'
	interface Props {
		md: string
		noPadding?: boolean
	}

	let { md, noPadding }: Props = $props()
	const plugins: Plugin[] = [
		gfmPlugin(),
		{ rehypePlugin: [rehypeRaw] },
		{ rehypePlugin: [rehypeGithubAlerts] }
	]
</script>

<div class="!prose-xs {noPadding ? '' : 'pgap'}">
	<Markdown {md} {plugins} />
</div>

<style global>
	.pgap > p {
		margin-top: 0.5rem;
		margin-bottom: 0.5rem;
	}

	:root {
		--github-alert-default-color: rgb(208, 215, 222);
		--github-alert-note-color: rgb(9, 105, 218);
		--github-alert-tip-color: rgb(26, 127, 55);
		--github-alert-important-color: rgb(130, 80, 223);
		--github-alert-warning-color: rgb(191, 135, 0);
		--github-alert-caution-color: rgb(207, 34, 46);
	}

	@media (prefers-color-scheme: dark) {
		:root {
			--github-alert-default-color: rgb(48, 54, 61);
			--github-alert-note-color: rgb(31, 111, 235);
			--github-alert-tip-color: rgb(35, 134, 54);
			--github-alert-important-color: rgb(137, 87, 229);
			--github-alert-warning-color: rgb(158, 106, 3);
			--github-alert-caution-color: rgb(248, 81, 73);
		}
	}

	.markdown-alert {
		padding: 0.5rem 1rem;
		margin-bottom: 16px;
		border-left: 0.25em solid var(--github-alert-default-color);
	}

	.markdown-alert > :first-child {
		margin-top: 0;
	}

	.markdown-alert > :last-child {
		margin-bottom: 0;
	}

	.markdown-alert-note {
		border-left-color: var(--github-alert-note-color);
	}

	.markdown-alert-tip {
		border-left-color: var(--github-alert-tip-color);
	}

	.markdown-alert-important {
		border-left-color: var(--github-alert-important-color);
	}

	.markdown-alert-warning {
		border-left-color: var(--github-alert-warning-color);
	}

	.markdown-alert-caution {
		border-left-color: var(--github-alert-caution-color);
	}

	.markdown-alert-title {
		display: flex;
		margin-bottom: 4px;
		align-items: center;
	}

	.markdown-alert-title > svg {
		margin-right: 8px;
	}

	.markdown-alert-note .markdown-alert-title {
		color: var(--github-alert-note-color);
	}

	.markdown-alert-tip .markdown-alert-title {
		color: var(--github-alert-tip-color);
	}

	.markdown-alert-important .markdown-alert-title {
		color: var(--github-alert-important-color);
	}

	.markdown-alert-warning .markdown-alert-title {
		color: var(--github-alert-warning-color);
	}

	.markdown-alert-caution .markdown-alert-title {
		color: var(--github-alert-caution-color);
	}
</style>
