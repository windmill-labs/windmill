<script lang="ts">
	import MarkdownCodeBlock from '$lib/components/MarkdownCodeBlock.svelte'
</script>

<!-- The viewer's code block without the chat context its own renderer needs. Mermaid and display
     blocks are on as in the viewer: only the session's own artifacts are exported. -->
<MarkdownCodeBlock renderMermaid lightMermaid renderDisplay staticDisplay />
