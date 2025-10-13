<script lang="ts">
    interface Props {
        height?: string;
        width?: string;
    }

    let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" {width} {height} viewBox="0 0 62.42 62.42">
    <defs>
        <linearGradient id="New_Gradient_Swatch_1" x1="64.01" y1="30.27" x2="32.99" y2="54.48" gradientUnits="userSpaceOnUse">
            <stop offset="0.18" stop-color="#0052cc" />
            <stop offset="1" stop-color="#2684ff" />
        </linearGradient>
    </defs>
    <title>Bitbucket-blue</title>
    <g id="Layer_2" data-name="Layer 2" >
        <g id="Blue" transform="translate(0 -3.13)">
            <path d="M2,6.26A2,2,0,0,0,0,8.58L8.49,60.12a2.72,2.72,0,0,0,2.66,2.27H51.88a2,2,0,0,0,2-1.68L62.37,8.59a2,2,0,0,0-2-2.32ZM37.75,43.51h-13L21.23,25.12H40.9Z" fill="#2684ff" />
            <path d="M59.67,25.12H40.9L37.75,43.51h-13L9.4,61.73a2.71,2.71,0,0,0,1.75.66H51.89a2,2,0,0,0,2-1.68Z" fill="url(#New_Gradient_Swatch_1)"/>
        </g>
    </g>
</svg>