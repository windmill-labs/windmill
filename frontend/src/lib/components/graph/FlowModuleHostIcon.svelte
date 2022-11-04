<script lang="ts">
	import type { ModuleHost } from '.'

	export let host: ModuleHost
	export let width = 16
	export let height = 16
	export let scale = 1
</script>

<svg
	width={width * scale}
	height={height * scale}
	{...$$restProps}
	viewBox="0 0 17 16"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	{#if host === 'inline'}
		<path
			d="M12.4998 1.33331H4.49984C3.76346 1.33331 3.1665 1.93027 3.1665 2.66665V13.3333C3.1665 14.0697 3.76346 14.6666 4.49984 14.6666H12.4998C13.2362 14.6666 13.8332 14.0697 13.8332 13.3333V2.66665C13.8332 1.93027 13.2362 1.33331 12.4998 1.33331Z"
			stroke="#595959"
			stroke-width="1"
			stroke-linecap="round"
			stroke-linejoin="round"
		/>
		<path
			d="M6.5 14.6667V12H10.5V14.6667"
			stroke="#595959"
			stroke-width="1"
			stroke-linecap="round"
			stroke-linejoin="round"
		/>
		<path
			d="M5.8335 4H5.84183"
			stroke="#595959"
			stroke-width="1"
			stroke-linecap="round"
			stroke-linejoin="round"
		/>
		<path
			d="M11.1665 4H11.1748"
			stroke="#595959"
			stroke-width="1"
			stroke-linecap="round"
			stroke-linejoin="round"
		/>
		<path
			d="M8.5 4H8.50833"
			stroke="#595959"
			stroke-width="1"
			stroke-linecap="round"
			stroke-linejoin="round"
		/>
		<path
			d="M8.5 6.66669H8.50833"
			stroke="#595959"
			stroke-width="1"
			stroke-linecap="round"
			stroke-linejoin="round"
		/>
		<path
			d="M8.5 9.33331H8.50833"
			stroke="#595959"
			stroke-width="1"
			stroke-linecap="round"
			stroke-linejoin="round"
		/>
		<path
			d="M11.1665 6.66669H11.1748"
			stroke="#595959"
			stroke-width="1"
			stroke-linecap="round"
			stroke-linejoin="round"
		/>
		<path
			d="M11.1665 9.33331H11.1748"
			stroke="#595959"
			stroke-width="1"
			stroke-linecap="round"
			stroke-linejoin="round"
		/>
		<path
			d="M5.8335 6.66669H5.84183"
			stroke="#595959"
			stroke-width="1"
			stroke-linecap="round"
			stroke-linejoin="round"
		/>
		<path
			d="M5.8335 9.33331H5.84183"
			stroke="#595959"
			stroke-width="1"
			stroke-linecap="round"
			stroke-linejoin="round"
		/>
	{:else}
		<path
			d="M8.42363 1.20868L7.14426 3.33305L6.33301 4.6793H9.88863L11.123 2.6293L11.9793 1.20868H8.42363Z"
			fill="#BCD4FC"
		/>
		<path
			d="M14.2218 4.67929H11.663H6.33301L8.11113 7.75867H9.88488H14.288H15.9999L14.2218 4.67929Z"
			fill="#3B82F6"
		/>
		<path
			d="M8.55814 8.59932L7.27877 10.8156L4.61377 15.4312H8.16939L9.05627 13.8949L11.2581 10.0812L12.1138 8.59869L8.55814 8.59932Z"
			fill="#3B82F6"
		/>
		<path
			d="M7.60615 8.56681L6.32678 6.35056L3.66178 1.73431L1.88428 4.81431L2.77115 6.35056L4.97303 10.1637L5.82865 11.6462L7.60615 8.56681Z"
			fill="#3B82F6"
		/>
		<path
			d="M4.05062 8.5668L1.57187 8.52056L0 8.49118L1.7775 11.5706L4.17063 11.6149L5.82875 11.6456L4.05062 8.5668Z"
			fill="#BCD4FC"
		/>
		<path
			d="M10.3364 11.6787L11.5364 13.8487L12.2971 15.2243L14.0746 12.1449L12.9164 10.0506L12.1146 8.59869L10.3364 11.6787Z"
			fill="#BCD4FC"
		/>
	{/if}
</svg>
