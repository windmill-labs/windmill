export function resolveBucketEndpointConfig(input) {
    const { bucketEndpoint = false, forcePathStyle = false, useAccelerateEndpoint = false, useArnRegion = false, disableMultiregionAccessPoints = false, } = input;
    return {
        ...input,
        bucketEndpoint,
        forcePathStyle,
        useAccelerateEndpoint,
        useArnRegion: typeof useArnRegion === "function" ? useArnRegion : () => Promise.resolve(useArnRegion),
        disableMultiregionAccessPoints: typeof disableMultiregionAccessPoints === "function"
            ? disableMultiregionAccessPoints
            : () => Promise.resolve(disableMultiregionAccessPoints),
    };
}
