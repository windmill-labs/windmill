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

<!-- Microsoft's Outlook-Icon-FY26 asset (cdn-dynmedia-1.microsoft.com, served on microsoft.com/microsoft-365/outlook), paths and gradient stops verbatim.
	 Microsoft's trademark guidelines forbid altering their brand assets, so the full-colour mark ships unchanged in both themes. -->
<svg
	width={effectiveWidth}
	height={effectiveHeight}
	viewBox="0 0 48 48"
	xmlns="http://www.w3.org/2000/svg"
	fill="none"
>
<path d="M30.9322 9.34307L7.97588 23.8943L6.00165 20.7797V18.0959C6.00165 17.1188 6.49634 16.2081 7.31603 15.6762L20.6606 7.01716C22.6937 5.69792 25.3125 5.69771 27.3458 7.01665L30.9322 9.34307Z" fill="url(#ms-outlook-0)"/>
<path d="M27.1402 6.88928C27.2092 6.93033 27.2777 6.97284 27.3455 7.01682L37.76 13.7724L11.9375 30.1404L7.97498 23.8891L26.9262 11.8533C28.7213 10.7133 28.7999 8.14871 27.1402 6.88928Z" fill="url(#ms-outlook-1)"/>
<path d="M27.1402 6.88928C27.2092 6.93033 27.2777 6.97284 27.3455 7.01682L37.76 13.7724L11.9375 30.1404L7.97498 23.8891L26.9262 11.8533C28.7213 10.7133 28.7999 8.14871 27.1402 6.88928Z" fill="url(#ms-outlook-2)" fill-opacity="0.2"/>
<path d="M22.2402 33.2659L11.9377 30.1406L33.842 16.2557C35.6868 15.0864 35.682 12.3932 33.8331 11.2304L33.7344 11.1684L34.0185 11.3451L40.685 15.6695C41.5049 16.2014 41.9998 17.1122 41.9998 18.0895V20.687L22.2402 33.2659Z" fill="url(#ms-outlook-3)"/>
<path d="M22.2402 33.2659L11.9377 30.1406L33.842 16.2557C35.6868 15.0864 35.682 12.3932 33.8331 11.2304L33.7344 11.1684L34.0185 11.3451L40.685 15.6695C41.5049 16.2014 41.9998 17.1122 41.9998 18.0895V20.687L22.2402 33.2659Z" fill="url(#ms-outlook-4)" fill-opacity="0.2"/>
<path d="M27.3458 7.01665C25.3125 5.69771 22.6937 5.69792 20.6605 7.01716L7.31603 15.6762C6.49634 16.2081 6.00165 17.1188 6.00165 18.0959V18.2272C6.03382 19.208 6.55002 20.1121 7.38373 20.6378L23.9764 31.0985L40.6108 20.6537C41.4747 20.1112 41.999 19.1628 41.999 18.1427V20.6873L41.9994 18.0896C41.9994 17.1123 41.5045 16.2015 40.6846 15.6696L27.3458 7.01665Z" fill="url(#ms-outlook-5)"/>
<path d="M21.0513 42.0035H35.748C39.1998 42.0035 41.998 39.2053 41.998 35.7535L41.998 18.1426C41.998 19.1627 41.4736 20.1111 40.6098 20.6536L18.7495 34.3798C17.5703 35.1202 16.8546 36.4149 16.8547 37.8073C16.8549 40.1248 18.7337 42.0035 21.0513 42.0035Z" fill="url(#ms-outlook-6)"/>
<path d="M21.0513 42.0035H35.748C39.1998 42.0035 41.998 39.2053 41.998 35.7535L41.998 18.1426C41.998 19.1627 41.4736 20.1111 40.6098 20.6536L18.7495 34.3798C17.5703 35.1202 16.8546 36.4149 16.8547 37.8073C16.8549 40.1248 18.7337 42.0035 21.0513 42.0035Z" fill="url(#ms-outlook-7)" fill-opacity="0.4"/>
<path d="M21.0513 42.0035H35.748C39.1998 42.0035 41.998 39.2053 41.998 35.7535L41.998 18.1426C41.998 19.1627 41.4736 20.1111 40.6098 20.6536L18.7495 34.3798C17.5703 35.1202 16.8546 36.4149 16.8547 37.8073C16.8549 40.1248 18.7337 42.0035 21.0513 42.0035Z" fill="url(#ms-outlook-8)" fill-opacity="0.5"/>
<path d="M27.0268 42.0023H12.2492C8.79745 42.0023 5.99923 39.2041 5.99923 35.7523V18.1297C5.99923 19.1478 6.52165 20.0948 7.38292 20.6377L29.2215 34.4058C30.4171 35.1595 31.1423 36.4741 31.1421 37.8874C31.1419 40.1601 29.2995 42.0023 27.0268 42.0023Z" fill="url(#ms-outlook-9)"/>
<path d="M27.0268 42.0023H12.2492C8.79745 42.0023 5.99923 39.2041 5.99923 35.7523V18.1297C5.99923 19.1478 6.52165 20.0948 7.38292 20.6377L29.2215 34.4058C30.4171 35.1595 31.1423 36.4741 31.1421 37.8874C31.1419 40.1601 29.2995 42.0023 27.0268 42.0023Z" fill="url(#ms-outlook-10)"/>
<rect x="4" y="23" width="16" height="16" rx="3.25" fill="url(#ms-outlook-11)"/>
<rect x="4" y="23" width="16" height="16" rx="3.25" fill="url(#ms-outlook-12)" fill-opacity="0.5"/>
<path d="M11.959 35.5999C10.636 35.5999 9.54994 35.186 8.70069 34.3583C7.85144 33.5306 7.42682 32.4505 7.42682 31.1179C7.42682 29.7107 7.85785 28.5726 8.7199 27.7035C9.58195 26.8345 10.7107 26.3999 12.1062 26.3999C13.4249 26.3999 14.4982 26.8158 15.3261 27.6477C16.1583 28.4795 16.5744 29.5762 16.5744 30.9378C16.5744 32.3367 16.1433 33.4644 15.2813 34.3211C14.4235 35.1736 13.3161 35.5999 11.959 35.5999ZM11.9974 33.8431C12.7186 33.8431 13.299 33.5968 13.7386 33.1044C14.1781 32.6119 14.3979 31.9269 14.3979 31.0496C14.3979 30.1349 14.1845 29.4231 13.7578 28.9141C13.331 28.405 12.7613 28.1505 12.0486 28.1505C11.3146 28.1505 10.7235 28.4133 10.2754 28.9389C9.82733 29.4604 9.60328 30.1515 9.60328 31.0123C9.60328 31.8856 9.82733 32.5767 10.2754 33.0857C10.7235 33.5906 11.2975 33.8431 11.9974 33.8431Z" fill="white"/>
<path d="M11.9554 35.7233C10.6375 35.7233 9.55551 35.2982 8.7095 34.448C7.8635 33.5977 7.44049 32.4881 7.44049 31.1192C7.44049 29.6737 7.86987 28.5046 8.72864 27.6119C9.5874 26.7191 10.7119 26.2727 12.102 26.2727C13.4157 26.2727 14.4849 26.7 15.3097 27.5545C16.1387 28.409 16.5532 29.5356 16.5532 30.9343C16.5532 32.3712 16.1238 33.5297 15.265 34.4097C14.4105 35.2855 13.3073 35.7233 11.9554 35.7233ZM11.9936 33.9187C12.7121 33.9187 13.2903 33.6657 13.7282 33.1598C14.1661 32.6539 14.385 31.9503 14.385 31.049C14.385 30.1095 14.1724 29.3783 13.7473 28.8554C13.3222 28.3325 12.7546 28.071 12.0447 28.071C11.3134 28.071 10.7246 28.341 10.2782 28.8809C9.83185 29.4165 9.60865 30.1265 9.60865 31.0108C9.60865 31.9078 9.83185 32.6178 10.2782 33.1407C10.7246 33.6593 11.2964 33.9187 11.9936 33.9187Z" fill="white"/>
<defs>
<linearGradient id="ms-outlook-0" x1="9.98908" y1="22.3649" x2="30.9322" y2="9.37495" gradientUnits="userSpaceOnUse">
<stop stop-color="#20A7FA"/>
<stop offset="0.4" stop-color="#3BD5FF"/>
<stop offset="1" stop-color="#C4B0FF"/>
</linearGradient>
<linearGradient id="ms-outlook-1" x1="17.1972" y1="26.7945" x2="28.8562" y2="8.12575" gradientUnits="userSpaceOnUse">
<stop stop-color="#165AD9"/>
<stop offset="0.5008" stop-color="#1880E5"/>
<stop offset="1" stop-color="#8587FF"/>
</linearGradient>
<linearGradient id="ms-outlook-2" x1="25.7005" y1="27.0484" x2="12.7563" y2="16.5013" gradientUnits="userSpaceOnUse">
<stop offset="0.236946" stop-color="#448AFF" stop-opacity="0"/>
<stop offset="0.792113" stop-color="#0032B1"/>
</linearGradient>
<linearGradient id="ms-outlook-3" x1="24.0534" y1="31.1099" x2="44.51" y2="18.0177" gradientUnits="userSpaceOnUse">
<stop stop-color="#1A43A6"/>
<stop offset="0.492267" stop-color="#2052CB"/>
<stop offset="1" stop-color="#5F20CB"/>
</linearGradient>
<linearGradient id="ms-outlook-4" x1="29.8281" y1="30.3273" x2="17.3975" y2="19.5708" gradientUnits="userSpaceOnUse">
<stop stop-color="#0045B9" stop-opacity="0"/>
<stop offset="0.669859" stop-color="#0D1F69"/>
</linearGradient>
<radialGradient id="ms-outlook-5" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(24.0018 6.81788) rotate(-90) scale(27.0027 29.2262)">
<stop offset="0.568182" stop-color="#275FF0" stop-opacity="0"/>
<stop offset="0.992424" stop-color="#002177"/>
</radialGradient>
<linearGradient id="ms-outlook-6" x1="41.998" y1="29.9431" x2="23.8517" y2="29.9431" gradientUnits="userSpaceOnUse">
<stop stop-color="#4DC4FF"/>
<stop offset="0.196145" stop-color="#0FAFFF"/>
</linearGradient>
<radialGradient id="ms-outlook-7" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(28.0928 37.9117) rotate(-45) scale(11.572)">
<stop offset="0.259477" stop-color="#0060D1"/>
<stop offset="0.908166" stop-color="#0383F1" stop-opacity="0"/>
</radialGradient>
<radialGradient id="ms-outlook-8" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(10.6314 46.472) rotate(-52.6577) scale(39.2814 35.5204)">
<stop offset="0.732317" stop-color="#F4A7F7" stop-opacity="0"/>
<stop offset="1" stop-color="#F4A7F7"/>
</radialGradient>
<radialGradient id="ms-outlook-9" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(18.5708 27.5319) rotate(123.339) scale(20.7258 53.7859)">
<stop stop-color="#49DEFF"/>
<stop offset="0.724349" stop-color="#29C3FF"/>
</radialGradient>
<linearGradient id="ms-outlook-10" x1="3.45756" y1="37.8723" x2="20.9291" y2="37.8597" gradientUnits="userSpaceOnUse">
<stop offset="0.205882" stop-color="#6CE0FF"/>
<stop offset="0.535" stop-color="#50D5FF" stop-opacity="0"/>
</linearGradient>
<radialGradient id="ms-outlook-11" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(3.94291 23.6154) rotate(46.9242) scale(21.0618)">
<stop offset="0.038877" stop-color="#0091FF"/>
<stop offset="0.919119" stop-color="#183DAD"/>
</radialGradient>
<radialGradient id="ms-outlook-12" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(12 32.7439) rotate(90) scale(11.2 12.9188)">
<stop offset="0.557796" stop-color="#0FA5F7" stop-opacity="0"/>
<stop offset="1" stop-color="#74C6FF"/>
</radialGradient>
</defs>
</svg>
