<script lang="ts">
	interface Props {
		height?: string
		width?: string
		size?: number
	}

	let { height, width, size = 24 }: Props = $props()
	let effectiveWidth = $derived(width ?? `${size}px`)
	let effectiveHeight = $derived(height ?? `${size}px`)
</script>

<!-- Microsoft's Word-Icon-FY26 asset (cdn-dynmedia-1.microsoft.com), paths and gradient stops verbatim.
	 Microsoft's trademark guidelines forbid altering their brand assets, so the full-colour mark ships unchanged in both themes. -->
<svg
	width={effectiveWidth}
	height={effectiveHeight}
	viewBox="0 0 48 48"
	xmlns="http://www.w3.org/2000/svg"
	fill="none"
>
<path d="M9 33.0901L23 16L39 27.1052V38.5C39 40.433 37.433 42 35.5 42H15C11.6863 42 9 39.3137 9 36V33.0901Z" fill="url(#ms-word-0)"/>
<path d="M9 21.0385C9 18.5532 11.0147 16.5385 13.5 16.5385H33.8889L39 14V26.5C39 28.433 37.433 30 35.5 30H15C11.6863 30 9 32.6863 9 36V21.0385Z" fill="url(#ms-word-1)"/>
<path d="M9 21.0385C9 18.5532 11.0147 16.5385 13.5 16.5385H33.8889L39 14V26.5C39 28.433 37.433 30 35.5 30H15C11.6863 30 9 32.6863 9 36V21.0385Z" fill="url(#ms-word-2)" fill-opacity="0.6"/>
<path d="M9 21.0385C9 18.5532 11.0147 16.5385 13.5 16.5385H33.8889L39 14V26.5C39 28.433 37.433 30 35.5 30H15C11.6863 30 9 32.6863 9 36V21.0385Z" fill="url(#ms-word-3)" fill-opacity="0.1"/>
<path d="M9 12C9 8.68629 11.6863 6 15 6H35.5C37.433 6 39 7.567 39 9.5V14.5C39 16.433 37.433 18 35.5 18H15C11.6863 18 9 20.6863 9 24V12Z" fill="url(#ms-word-4)"/>
<path d="M9 12C9 8.68629 11.6863 6 15 6H35.5C37.433 6 39 7.567 39 9.5V14.5C39 16.433 37.433 18 35.5 18H15C11.6863 18 9 20.6863 9 24V12Z" fill="url(#ms-word-5)" fill-opacity="0.8"/>
<rect x="4" y="23" width="16" height="16" rx="3.25" fill="url(#ms-word-6)"/>
<rect x="4" y="23" width="16" height="16" rx="3.25" fill="url(#ms-word-7)" fill-opacity="0.65"/>
<path d="M17.4859 26.4286L15.5167 35.5703L13.1628 35.5714L12.0002 30.0857L10.7832 35.5714H8.40686L6.51451 26.4297H8.45546L9.62308 32.4629L10.7832 26.4297H13.1628L14.3774 32.4629L15.5167 26.4297L17.4859 26.4286Z" fill="white"/>
<defs>
<radialGradient id="ms-word-0" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(38.1254 41.9993) scale(47.5733 20.1494)">
<stop offset="0.180414" stop-color="#1657F4"/>
<stop offset="0.574542" stop-color="#0036C4"/>
</radialGradient>
<linearGradient id="ms-word-1" x1="9" y1="25" x2="31.9747" y2="25" gradientUnits="userSpaceOnUse">
<stop stop-color="#66C0FF"/>
<stop offset="0.25606" stop-color="#0094F0"/>
</linearGradient>
<radialGradient id="ms-word-2" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(39 14.7158) rotate(133.546) scale(29.3648 72.3194)">
<stop offset="0.140259" stop-color="#D471FF"/>
<stop offset="0.830508" stop-color="#509DF5" stop-opacity="0"/>
</radialGradient>
<radialGradient id="ms-word-3" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(34.6111 29.2308) rotate(90) scale(18.6154 101.647)">
<stop offset="0.283333" stop-color="#4F006F" stop-opacity="0"/>
<stop offset="1" stop-color="#4F006F"/>
</radialGradient>
<linearGradient id="ms-word-4" x1="9" y1="15" x2="38.9918" y2="15.4954" gradientUnits="userSpaceOnUse">
<stop stop-color="#9DEAFF"/>
<stop offset="0.201276" stop-color="#3BD5FF"/>
</linearGradient>
<radialGradient id="ms-word-5" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(39 6.58563) rotate(166.855) scale(29.4949 70.6435)">
<stop offset="0.0606583" stop-color="#E4A7FE"/>
<stop offset="0.538869" stop-color="#E4A7FE" stop-opacity="0"/>
</radialGradient>
<radialGradient id="ms-word-6" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(4 23) rotate(45) scale(22.6274)">
<stop offset="0.0811439" stop-color="#367AF2"/>
<stop offset="0.871875" stop-color="#001A8F"/>
</radialGradient>
<radialGradient id="ms-word-7" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(12 32.6) rotate(90) scale(11.2 12.765)">
<stop offset="0.586954" stop-color="#2763E5" stop-opacity="0"/>
<stop offset="0.973806" stop-color="#58AAFE"/>
</radialGradient>
</defs>
</svg>
