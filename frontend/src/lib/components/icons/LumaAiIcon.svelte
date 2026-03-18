<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
<g clip-path="url(#a)">
		<path d="m4.108 9.033 12.066-6.965v27.865L4.108 22.967V9.033Z" fill="#000"/>
		<path d="M16.174 29.933 4.108 22.967 16.174 16l12.066 6.966-12.066 6.967Z" fill="url(#b)"/>
		<path d="M16.174 29.933 4.108 22.967 16.174 16l12.066 6.966-12.066 6.967Z" fill="url(#c)"/>
		<path d="m4.108 9.033 12.066-6.965v27.865L4.108 22.967V9.033Z" fill="url(#d)" style="mix-blend-mode:screen"/>
		<path d="M16.174 29.933 4.108 22.967 16.174 16l12.066 6.966-12.066 6.967Z" fill="url(#e)" style="mix-blend-mode:overlay"/>
		<path d="m4.108 9.033 12.066-6.965v27.865L4.108 22.967V9.033Z" fill="url(#f)" style="mix-blend-mode:overlay"/>
	</g>
	<defs>
		<linearGradient id="b" x1="4.108" x2="28.24" y1="22.966" y2="22.966" gradientUnits="userSpaceOnUse">
			<stop stop-color="#00A"/>
			<stop offset="1" stop-color="#A78DFF"/>
		</linearGradient>
		<linearGradient id="c" x1="4.108" x2="28.24" y1="22.966" y2="22.966" gradientUnits="userSpaceOnUse">
			<stop stop-color="#00A"/>
			<stop offset="1" stop-color="#A78DFF"/>
		</linearGradient>
		<linearGradient id="d" x1="17.747" x2="7.211" y1="28.357" y2="6.416" gradientUnits="userSpaceOnUse">
			<stop stop-color="#004EFF"/>
			<stop offset="1" stop-color="#0FF"/>
		</linearGradient>
		<linearGradient id="e" x1="4.108" x2="28.24" y1="22.966" y2="22.966" gradientUnits="userSpaceOnUse">
			<stop stop-color="#00A"/>
			<stop offset="1" stop-color="#A78DFF"/>
		</linearGradient>
		<linearGradient id="f" x1="17.747" x2="7.211" y1="28.357" y2="6.416" gradientUnits="userSpaceOnUse">
			<stop stop-color="#004EFF"/>
			<stop offset="1" stop-color="#0FF"/>
		</linearGradient>
		<clipPath id="a">
			<path d="M0 0h24.348v28H0z" fill="#fff" transform="translate(4 2)"/>
		</clipPath>
	</defs>
</svg>
