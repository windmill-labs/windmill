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

<!-- Microsoft's OneDrive-Icon-FY26 asset (cdn-dynmedia-1.microsoft.com), paths and gradient stops verbatim.
	 Microsoft's trademark guidelines forbid altering their brand assets, so the full-colour mark ships unchanged in both themes. -->
<svg
	width={effectiveWidth}
	height={effectiveHeight}
	viewBox="0 0 48 48"
	xmlns="http://www.w3.org/2000/svg"
	fill="none"
>
<path d="M14.3386 13.6726C7.73405 13.6728 2.7972 19.0791 2.42554 25.1018C2.65561 26.3995 3.41182 28.9621 4.59605 28.8305C6.07633 28.666 9.80446 28.8305 12.9843 23.0738C15.3072 18.8685 20.0856 13.6725 14.3386 13.6726Z" fill="url(#ms-onedrive-0)"/>
<path d="M12.8114 15.9208C10.5913 19.4358 7.60292 24.4724 6.59433 26.0573C5.39543 27.9413 2.22036 27.1409 2.48339 24.4405C2.45746 24.6597 2.43754 24.8807 2.42381 25.1032C1.98946 32.1289 7.55988 37.9635 14.4916 37.9635C22.1313 37.9635 40.3512 28.4448 38.5073 18.9074C36.5639 13.3012 31.1015 9.28009 24.911 9.28009C18.7205 9.28009 14.753 12.8466 12.8114 15.9208Z" fill="url(#ms-onedrive-1)"/>
<path d="M12.8114 15.9208C10.5913 19.4358 7.60292 24.4724 6.59433 26.0573C5.39543 27.9413 2.22036 27.1409 2.48339 24.4405C2.45746 24.6597 2.43754 24.8807 2.42381 25.1032C1.98946 32.1289 7.55988 37.9635 14.4916 37.9635C22.1313 37.9635 40.3512 28.4448 38.5073 18.9074C36.5639 13.3012 31.1015 9.28009 24.911 9.28009C18.7205 9.28009 14.753 12.8466 12.8114 15.9208Z" fill="url(#ms-onedrive-2)" fill-opacity="0.4"/>
<path d="M12.8114 15.9208C10.5913 19.4358 7.60292 24.4724 6.59433 26.0573C5.39543 27.9413 2.22036 27.1409 2.48339 24.4405C2.45746 24.6597 2.43754 24.8807 2.42381 25.1032C1.98946 32.1289 7.55988 37.9635 14.4916 37.9635C22.1313 37.9635 40.3512 28.4448 38.5073 18.9074C36.5639 13.3012 31.1015 9.28009 24.911 9.28009C18.7205 9.28009 14.753 12.8466 12.8114 15.9208Z" fill="url(#ms-onedrive-3)"/>
<path d="M12.8114 15.9208C10.5913 19.4358 7.60292 24.4724 6.59433 26.0573C5.39543 27.9413 2.22036 27.1409 2.48339 24.4405C2.45746 24.6597 2.43754 24.8807 2.42381 25.1032C1.98946 32.1289 7.55988 37.9635 14.4916 37.9635C22.1313 37.9635 40.3512 28.4448 38.5073 18.9074C36.5639 13.3012 31.1015 9.28009 24.911 9.28009C18.7205 9.28009 14.753 12.8466 12.8114 15.9208Z" fill="url(#ms-onedrive-4)" fill-opacity="0.6"/>
<path d="M12.8114 15.9208C10.5913 19.4358 7.60292 24.4724 6.59433 26.0573C5.39543 27.9413 2.22036 27.1409 2.48339 24.4405C2.45746 24.6597 2.43754 24.8807 2.42381 25.1032C1.98946 32.1289 7.55988 37.9635 14.4916 37.9635C22.1313 37.9635 40.3512 28.4448 38.5073 18.9074C36.5639 13.3012 31.1015 9.28009 24.911 9.28009C18.7205 9.28009 14.753 12.8466 12.8114 15.9208Z" fill="url(#ms-onedrive-5)" fill-opacity="0.9"/>
<path d="M14.38 37.9665C14.38 37.9665 32.6214 38.0024 35.7155 38.0024C41.3306 38.0024 45.5999 33.4181 45.6 28.0688C45.6 22.7194 41.2449 18.1629 35.7155 18.1629C30.1861 18.1629 27.0019 22.2996 24.6102 26.815C21.8077 32.1062 18.2326 37.903 14.38 37.9665Z" fill="url(#ms-onedrive-6)"/>
<path d="M14.38 37.9665C14.38 37.9665 32.6214 38.0024 35.7155 38.0024C41.3306 38.0024 45.5999 33.4181 45.6 28.0688C45.6 22.7194 41.2449 18.1629 35.7155 18.1629C30.1861 18.1629 27.0019 22.2996 24.6102 26.815C21.8077 32.1062 18.2326 37.903 14.38 37.9665Z" fill="url(#ms-onedrive-7)" fill-opacity="0.4"/>
<path d="M14.38 37.9665C14.38 37.9665 32.6214 38.0024 35.7155 38.0024C41.3306 38.0024 45.5999 33.4181 45.6 28.0688C45.6 22.7194 41.2449 18.1629 35.7155 18.1629C30.1861 18.1629 27.0019 22.2996 24.6102 26.815C21.8077 32.1062 18.2326 37.903 14.38 37.9665Z" fill="url(#ms-onedrive-8)" fill-opacity="0.9"/>
<defs>
<radialGradient id="ms-onedrive-0" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(3.24464 15.2511) rotate(50.1526) scale(13.6159 22.5845)">
<stop stop-color="#4894FE"/>
<stop offset="0.695072" stop-color="#0934B3"/>
</radialGradient>
<radialGradient id="ms-onedrive-1" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(39.7971 -0.425349) rotate(130.923) scale(58.5497 43.3858)">
<stop offset="0.165327" stop-color="#23C0FE"/>
<stop offset="0.534" stop-color="#1C91FF"/>
</radialGradient>
<radialGradient id="ms-onedrive-2" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(12.0798 16.0264) rotate(-139.986) scale(11.9037 27.2501)">
<stop stop-color="white"/>
<stop offset="0.660528" stop-color="#ADC0FF" stop-opacity="0"/>
</radialGradient>
<radialGradient id="ms-onedrive-3" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(25.0236 30.0957) rotate(-139.764) scale(13.4172 20.3769)">
<stop stop-color="#033ACC"/>
<stop offset="1" stop-color="#368EFF" stop-opacity="0"/>
</radialGradient>
<radialGradient id="ms-onedrive-4" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(11.2919 8.37262) rotate(66.5713) scale(29.4404 31.7826)">
<stop offset="0.592618" stop-color="#3464E3" stop-opacity="0"/>
<stop offset="1" stop-color="#033ACC"/>
</radialGradient>
<radialGradient id="ms-onedrive-5" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(39.3251 2.63231) rotate(135) scale(43.306 67.8459)">
<stop stop-color="#4BFDE8"/>
<stop offset="0.543937" stop-color="#4BFDE8" stop-opacity="0"/>
</radialGradient>
<linearGradient id="ms-onedrive-6" x1="29.9997" y1="37.9823" x2="29.9997" y2="18.3982" gradientUnits="userSpaceOnUse">
<stop stop-color="#0086FF"/>
<stop offset="0.49" stop-color="#00BBFF"/>
</linearGradient>
<radialGradient id="ms-onedrive-7" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(19.7659 20.4961) rotate(21.6324) scale(19.6236 37.1605)">
<stop stop-color="white"/>
<stop offset="0.785262" stop-color="white" stop-opacity="0"/>
</radialGradient>
<radialGradient id="ms-onedrive-8" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(44.9897 16.6252) rotate(139.243) scale(26.9034 26.9946)">
<stop stop-color="#4BFDE8"/>
<stop offset="0.584724" stop-color="#4BFDE8" stop-opacity="0"/>
</radialGradient>
</defs>
</svg>
