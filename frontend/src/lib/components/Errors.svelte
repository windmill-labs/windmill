<script lang="ts">
	type Props = {
		errors: string[]
	}

	let { errors }: Props = $props()
</script>

<div class="flex flex-col gap-2">
	{#if errors.length > 0}
		<ul class="list-disc pl-5 text-red-600">
			{#each errors as error}
				<li class="text-sm rounded-md py-1">
					{error}
				</li>
			{/each}
		</ul>
	{/if}
</div>
