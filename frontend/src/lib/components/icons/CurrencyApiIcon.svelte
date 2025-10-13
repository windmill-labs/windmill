<script lang="ts">
    interface Props {
        height?: string;
        width?: string;
    }

    let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg {width} {height} viewBox="0 0 50 50" fill="none" xmlns="http://www.w3.org/2000/svg">
    <rect width="50" height="50" fill="white"/>
    <path d="M42 25.4897C42 28.2017 39.7997 30.402 37.0877 30.402H11.9225C9.21055 30.402 7.01025 28.2017 7.01025 25.4897C7.01025 22.7777 9.21055 20.5774 11.9225 20.5774H37.0877C39.7997 20.5774 42 22.7777 42 25.4897Z" fill="url(#paint0_linear_682_2)"/>
    <path d="M37.1287 30.3713C39.8247 30.3713 42.0102 28.1858 42.0102 25.4897C42.0102 22.7937 39.8247 20.6082 37.1287 20.6082C34.4326 20.6082 32.2471 22.7937 32.2471 25.4897C32.2471 28.1858 34.4326 30.3713 37.1287 30.3713Z" fill="url(#paint1_linear_682_2)"/>
    <path d="M36.924 33.155H24.5C20.2632 33.155 16.8348 29.7266 16.8348 25.4898C16.8348 21.2529 20.2632 17.8246 24.5 17.8246H36.9137C39.5336 17.8246 41.8158 15.8494 41.9898 13.2398C42.174 10.3743 39.9123 8 37.0877 8H24.5C14.8392 8 7 15.8392 7 25.5C7 35.1608 14.8392 43 24.5 43H37.0877C39.9123 43 42.174 40.6155 41.9898 37.7602C41.826 35.1301 39.5439 33.155 36.924 33.155Z" fill="url(#paint2_linear_682_2)"/>
    <defs>
    <linearGradient id="paint0_linear_682_2" x1="43.8572" y1="25.5613" x2="16.6571" y2="25.4606" gradientUnits="userSpaceOnUse">
    <stop stop-color="#2994FF"/>
    <stop offset="1" stop-color="#0033BF"/>
    </linearGradient>
    <linearGradient id="paint1_linear_682_2" x1="41.6178" y1="25.4595" x2="3.25451" y2="25.7176" gradientUnits="userSpaceOnUse">
    <stop stop-color="#45A2FF"/>
    <stop offset="1" stop-color="#2660FF"/>
    </linearGradient>
    <linearGradient id="paint2_linear_682_2" x1="39.3177" y1="25.5792" x2="9.38529" y2="25.302" gradientUnits="userSpaceOnUse">
    <stop stop-color="#45A2FF"/>
    <stop offset="1" stop-color="#2660FF"/>
    </linearGradient>
    </defs>
    </svg>    