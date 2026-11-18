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

<!-- Microsoft's Excel-Icon-FY26 asset (cdn-dynmedia-1.microsoft.com), paths and gradient stops verbatim.
	 Microsoft's trademark guidelines forbid altering their brand assets, so the full-colour mark ships unchanged in both themes. -->
<svg
	width={effectiveWidth}
	height={effectiveHeight}
	viewBox="0 0 48 48"
	xmlns="http://www.w3.org/2000/svg"
	fill="none"
>
<path d="M9 17.5C9 15.0147 11.0147 13 13.5 13H39V39C39 40.6569 37.6569 42 36 42H15C11.6863 42 9 39.3137 9 36V17.5Z" fill="url(#ms-excel-0)"/>
<path d="M9 17.5C9 15.0147 11.0147 13 13.5 13H39V39C39 40.6569 37.6569 42 36 42H15C11.6863 42 9 39.3137 9 36V17.5Z" fill="url(#ms-excel-1)" fill-opacity="0.7"/>
<path d="M9 22.4999C9 20.0147 11.0147 17.9999 13.5 17.9999H27C25.3431 17.9999 24 19.3431 24 20.9999V26.9999C24 28.6568 22.6569 29.9999 21 29.9999H15C11.6863 29.9999 9 32.6862 9 35.9999V22.4999Z" fill="url(#ms-excel-2)"/>
<path d="M9 22.4999C9 20.0147 11.0147 17.9999 13.5 17.9999H27C25.3431 17.9999 24 19.3431 24 20.9999V26.9999C24 28.6568 22.6569 29.9999 21 29.9999H15C11.6863 29.9999 9 32.6862 9 35.9999V22.4999Z" fill="url(#ms-excel-3)" fill-opacity="0.3"/>
<path d="M9 12C9 8.68629 11.6863 6 15 6H27V18H15C11.6863 18 9 20.6863 9 24V12Z" fill="url(#ms-excel-4)"/>
<path d="M9 12C9 8.68629 11.6863 6 15 6H27V18H15C11.6863 18 9 20.6863 9 24V12Z" fill="url(#ms-excel-5)"/>
<path d="M9 12C9 8.68629 11.6863 6 15 6H27V18H15C11.6863 18 9 20.6863 9 24V12Z" fill="url(#ms-excel-6)"/>
<rect width="15" height="12" rx="3" transform="matrix(-1 0 0 1 39 6)" fill="url(#ms-excel-7)"/>
<rect x="4" y="23" width="16" height="16" rx="3.25" fill="url(#ms-excel-8)"/>
<rect x="4" y="23" width="16" height="16" rx="3.25" fill="url(#ms-excel-9)" fill-opacity="0.3"/>
<path d="M16.2056 35.5714H13.7393L12.1907 32.6577C12.1355 32.5557 12.093 32.4749 12.0633 32.4154C12.0378 32.3517 12.0102 32.2794 11.9804 32.1987H11.9549C11.9167 32.3007 11.8806 32.3836 11.8466 32.4473C11.8126 32.5111 11.7722 32.5897 11.7255 32.6832L10.1196 35.5714H7.79352L10.5848 30.9936L7.9847 26.4286H10.4191L11.7956 29.0299C11.8508 29.1362 11.8976 29.2297 11.9358 29.3104C11.9783 29.3869 12.0208 29.4783 12.0633 29.5846H12.0888C12.1482 29.4613 12.195 29.3636 12.229 29.2913C12.2672 29.2191 12.3182 29.1234 12.3819 29.0044L13.8094 26.4286H16.1291L13.4908 30.9235L16.2056 35.5714Z" fill="white"/>
<defs>
<radialGradient id="ms-excel-0" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(39 44.4167) rotate(-133.549) scale(41.1236 31.9082)">
<stop offset="0.0647233" stop-color="#379539"/>
<stop offset="0.422149" stop-color="#297C2D"/>
<stop offset="0.703448" stop-color="#15561C"/>
</radialGradient>
<radialGradient id="ms-excel-1" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(18.1667 23.4722) rotate(-135.971) scale(16.6611 12.8906)">
<stop stop-color="#073B10"/>
<stop offset="0.992234" stop-color="#084A13" stop-opacity="0"/>
</radialGradient>
<linearGradient id="ms-excel-2" x1="9" y1="26.9999" x2="22.7848" y2="26.9999" gradientUnits="userSpaceOnUse">
<stop stop-color="#52D17C"/>
<stop offset="0.328512" stop-color="#4AA647"/>
</linearGradient>
<linearGradient id="ms-excel-3" x1="22.5" y1="17.9999" x2="22.5" y2="30.5027" gradientUnits="userSpaceOnUse">
<stop stop-color="#29852F"/>
<stop offset="0.499968" stop-color="#4AA647" stop-opacity="0"/>
</linearGradient>
<linearGradient id="ms-excel-4" x1="9.66301" y1="20.5895" x2="26.2944" y2="6.16483" gradientUnits="userSpaceOnUse">
<stop stop-color="#66D052"/>
<stop offset="1" stop-color="#85E972"/>
</linearGradient>
<radialGradient id="ms-excel-5" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(26.352 13.6634) rotate(-180) scale(9.01883 19.094)">
<stop offset="0.292495" stop-color="#4EB43B"/>
<stop offset="1" stop-color="#72CC61" stop-opacity="0"/>
</radialGradient>
<linearGradient id="ms-excel-6" x1="17.1147" y1="14.3923" x2="9" y2="14.3923" gradientUnits="userSpaceOnUse">
<stop offset="0.183694" stop-color="#C0E075" stop-opacity="0"/>
<stop offset="1" stop-color="#D1EB95"/>
</linearGradient>
<radialGradient id="ms-excel-7" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(-1.91094 13.6794) rotate(-38.9697) scale(21.751 21.6904)">
<stop offset="0.439983" stop-color="#79E96D"/>
<stop offset="1" stop-color="#D0EB76"/>
</radialGradient>
<radialGradient id="ms-excel-8" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(4 23) rotate(45) scale(22.6274 64.4141)">
<stop stop-color="#20A85E"/>
<stop offset="0.94375" stop-color="#09442A"/>
</radialGradient>
<radialGradient id="ms-excel-9" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(12 32.6) rotate(90) scale(11.2 12.9)">
<stop offset="0.580357" stop-color="#33A662" stop-opacity="0"/>
<stop offset="0.973806" stop-color="#98F0B0"/>
</radialGradient>
</defs>
</svg>
