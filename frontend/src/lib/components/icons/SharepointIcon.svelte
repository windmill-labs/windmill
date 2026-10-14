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

<!-- Microsoft's SharePoint-Icon-FY26 asset (cdn-dynmedia-1.microsoft.com), paths and gradient stops verbatim.
	 Microsoft's trademark guidelines forbid altering their brand assets, so the full-colour mark ships unchanged in both themes. -->
<svg
	width={effectiveWidth}
	height={effectiveHeight}
	viewBox="0 0 48 48"
	xmlns="http://www.w3.org/2000/svg"
	fill="none"
>
<path d="M19 28C25.6274 28 31 22.6274 31 16C31 9.37258 25.6274 4 19 4C12.3726 4 7 9.37258 7 16C7 22.6274 12.3726 28 19 28Z" fill="url(#ms-sharepoint-0)"/>
<path d="M19 28C25.6274 28 31 22.6274 31 16C31 9.37258 25.6274 4 19 4C12.3726 4 7 9.37258 7 16C7 22.6274 12.3726 28 19 28Z" fill="url(#ms-sharepoint-1)" fill-opacity="0.2"/>
<path d="M19 28C25.6274 28 31 22.6274 31 16C31 9.37258 25.6274 4 19 4C12.3726 4 7 9.37258 7 16C7 22.6274 12.3726 28 19 28Z" fill="url(#ms-sharepoint-2)" fill-opacity="0.31"/>
<path d="M19 28C25.6274 28 31 22.6274 31 16C31 9.37258 25.6274 4 19 4C12.3726 4 7 9.37258 7 16C7 22.6274 12.3726 28 19 28Z" fill="url(#ms-sharepoint-3)" fill-opacity="0.7"/>
<path d="M29.9453 36C35.4682 36 39.9453 31.5228 39.9453 26C39.9453 20.4772 35.4682 16 29.9453 16C24.4225 16 19.9453 20.4772 19.9453 26C19.9453 31.5228 24.4225 36 29.9453 36Z" fill="url(#ms-sharepoint-4)"/>
<path d="M29.9453 36C35.4682 36 39.9453 31.5228 39.9453 26C39.9453 20.4772 35.4682 16 29.9453 16C24.4225 16 19.9453 20.4772 19.9453 26C19.9453 31.5228 24.4225 36 29.9453 36Z" fill="url(#ms-sharepoint-5)" fill-opacity="0.5"/>
<path d="M29.9453 36C35.4682 36 39.9453 31.5228 39.9453 26C39.9453 20.4772 35.4682 16 29.9453 16C24.4225 16 19.9453 20.4772 19.9453 26C19.9453 31.5228 24.4225 36 29.9453 36Z" fill="url(#ms-sharepoint-6)" fill-opacity="0.7"/>
<path d="M20.5 44C25.1944 44 29 40.1944 29 35.5C29 30.8056 25.1944 27 20.5 27C15.8056 27 12 30.8056 12 35.5C12 40.1944 15.8056 44 20.5 44Z" fill="url(#ms-sharepoint-7)"/>
<path d="M20.5 44C25.1944 44 29 40.1944 29 35.5C29 30.8056 25.1944 27 20.5 27C15.8056 27 12 30.8056 12 35.5C12 40.1944 15.8056 44 20.5 44Z" fill="url(#ms-sharepoint-8)" fill-opacity="0.32"/>
<rect x="4" y="23" width="16" height="16" rx="3.25" fill="url(#ms-sharepoint-9)"/>
<rect x="4" y="23" width="16" height="16" rx="3.25" fill="url(#ms-sharepoint-10)" fill-opacity="0.6"/>
<path d="M8.55579 33.6836L10.2892 32.8076C10.4846 33.1901 10.7395 33.4718 11.0539 33.6528C11.3725 33.8338 11.7209 33.9243 12.099 33.9243C12.5196 33.9243 12.8404 33.842 13.0613 33.6775C13.2823 33.5089 13.3927 33.2559 13.3927 32.9187C13.3927 32.6554 13.2865 32.4333 13.0741 32.2524C12.8617 32.0673 12.4857 31.9275 11.9461 31.8329C10.918 31.6519 10.1702 31.3352 9.70289 30.8828C9.2398 30.4304 9.00825 29.8669 9.00825 29.1924C9.00825 28.3534 9.31415 27.683 9.92593 27.1812C10.5377 26.6795 11.3449 26.4286 12.3476 26.4286C13.0231 26.4286 13.6179 26.5623 14.132 26.8296C14.646 27.0969 15.0539 27.4794 15.3555 27.9771L13.6604 28.8223C13.4734 28.5426 13.2716 28.3411 13.055 28.2177C12.8383 28.0902 12.5664 28.0264 12.2392 28.0264C11.8484 28.0264 11.551 28.1087 11.3471 28.2732C11.1474 28.4377 11.0475 28.6516 11.0475 28.9148C11.0475 29.141 11.1431 29.3405 11.3343 29.5132C11.5297 29.6818 11.9206 29.8196 12.5069 29.9266C13.4926 30.1075 14.2275 30.4366 14.7119 30.9136C15.2005 31.3866 15.4447 31.985 15.4447 32.7089C15.4447 33.5891 15.1537 34.2862 14.5717 34.8003C13.9896 35.3144 13.1591 35.5714 12.0799 35.5714C11.2982 35.5714 10.5929 35.4069 9.96417 35.0779C9.33964 34.7448 8.87018 34.28 8.55579 33.6836Z" fill="white"/>
<path d="M8.55322 33.772L10.2878 32.8665C10.4833 33.2618 10.7384 33.5531 11.053 33.7401C11.3718 33.9272 11.7204 34.0207 12.0988 34.0207C12.5197 34.0207 12.8407 33.9357 13.0617 33.7656C13.2828 33.5913 13.3933 33.3299 13.3933 32.9813C13.3933 32.7092 13.287 32.4796 13.0745 32.2925C12.8619 32.1012 12.4857 31.9567 11.9458 31.8589C10.9169 31.6719 10.1687 31.3445 9.70107 30.8769C9.23768 30.4092 9.00599 29.8268 9.00599 29.1296C9.00599 28.2623 9.31208 27.5694 9.92427 27.0507C10.5365 26.532 11.3442 26.2727 12.3475 26.2727C13.0235 26.2727 13.6186 26.4109 14.1331 26.6872C14.6475 26.9635 15.0556 27.3589 15.3574 27.8733L13.6612 28.747C13.4741 28.4579 13.2722 28.2496 13.0554 28.122C12.8385 27.9902 12.5665 27.9243 12.2391 27.9243C11.848 27.9243 11.5504 28.0094 11.3463 28.1794C11.1465 28.3495 11.0466 28.5705 11.0466 28.8426C11.0466 29.0764 11.1423 29.2826 11.3336 29.4612C11.5291 29.6355 11.9203 29.7779 12.5069 29.8884C13.4932 30.0755 14.2287 30.4156 14.7134 30.9088C15.2023 31.3976 15.4467 32.0162 15.4467 32.7644C15.4467 33.6742 15.1555 34.3948 14.5731 34.9262C13.9906 35.4576 13.1595 35.7233 12.0797 35.7233C11.2974 35.7233 10.5917 35.5533 9.96253 35.2132C9.33759 34.8688 8.86782 34.3884 8.55322 33.772Z" fill="white"/>
<defs>
<linearGradient id="ms-sharepoint-0" x1="10" y1="7" x2="26" y2="28" gradientUnits="userSpaceOnUse">
<stop stop-color="#00E3DF"/>
<stop offset="0.410156" stop-color="#0097A8"/>
<stop offset="1" stop-color="#007791"/>
</linearGradient>
<radialGradient id="ms-sharepoint-1" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(22.6718 31.3198) rotate(-112.448) scale(18.4261 33.1291)">
<stop offset="0.28573" stop-color="#003B5D"/>
<stop offset="0.612265" stop-color="#004A6C" stop-opacity="0.688298"/>
<stop offset="0.968041" stop-color="#006F94" stop-opacity="0"/>
</radialGradient>
<radialGradient id="ms-sharepoint-2" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(23.0954 26.0253) rotate(-112.063) scale(15.5519 27.9272)">
<stop offset="0.259744" stop-color="#002A42"/>
<stop offset="0.612265" stop-color="#004261" stop-opacity="0.688298"/>
<stop offset="0.968041" stop-color="#006F94" stop-opacity="0"/>
</radialGradient>
<radialGradient id="ms-sharepoint-3" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(25.8486 5.27069) rotate(124.447) scale(13.0107)">
<stop stop-color="#78EDFF"/>
<stop offset="1" stop-color="#2CCFCA" stop-opacity="0"/>
</radialGradient>
<linearGradient id="ms-sharepoint-4" x1="22.4453" y1="18.5" x2="35.7786" y2="36" gradientUnits="userSpaceOnUse">
<stop stop-color="#00E3DF"/>
<stop offset="0.476427" stop-color="#00A2B8"/>
<stop offset="0.945063" stop-color="#00637C"/>
</linearGradient>
<radialGradient id="ms-sharepoint-5" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(24.2161 34.3333) rotate(-70.8012) scale(12.3537 22.2587)">
<stop stop-color="#003B5D"/>
<stop offset="0.492035" stop-color="#004C6C" stop-opacity="0.72"/>
<stop offset="0.968041" stop-color="#007A86" stop-opacity="0"/>
</radialGradient>
<radialGradient id="ms-sharepoint-6" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(35.6525 17.0589) rotate(124.447) scale(10.8423)">
<stop stop-color="#78EDFF"/>
<stop offset="1" stop-color="#2CCFCA" stop-opacity="0"/>
</radialGradient>
<linearGradient id="ms-sharepoint-7" x1="14.55" y1="27.85" x2="23.05" y2="44.85" gradientUnits="userSpaceOnUse">
<stop offset="0.0534989" stop-color="#75FFF6"/>
<stop offset="0.51144" stop-color="#00C7D1"/>
<stop offset="0.96002" stop-color="#0096AD"/>
</linearGradient>
<linearGradient id="ms-sharepoint-8" x1="28.961" y1="44.0008" x2="24.4188" y2="37.6258" gradientUnits="userSpaceOnUse">
<stop offset="0.259744" stop-color="#0E5A5D"/>
<stop offset="0.535716" stop-color="#126C6B" stop-opacity="0.688298"/>
<stop offset="0.968041" stop-color="#1C948A" stop-opacity="0"/>
</linearGradient>
<radialGradient id="ms-sharepoint-9" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(4 23) rotate(45) scale(22.6274)">
<stop offset="0.0625" stop-color="#00B6BD"/>
<stop offset="0.890131" stop-color="#00495C"/>
</radialGradient>
<radialGradient id="ms-sharepoint-10" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(12 32.6) rotate(90) scale(11.2 12.75)">
<stop offset="0.566964" stop-color="#1E8581" stop-opacity="0"/>
<stop offset="0.973806" stop-color="#1ECBE6"/>
</radialGradient>
</defs>
</svg>
