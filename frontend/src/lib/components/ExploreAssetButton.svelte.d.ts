export declare function assetCanBeExplored(asset: Asset, _resourceMetadata?: {
    resource_type?: string;
}): boolean;
import { type Asset } from '$lib/components/assets/lib';
import { ButtonType } from '$lib/components/common';
import DbManagerDrawer from '$lib/components/DBManagerDrawer.svelte';
import S3FilePicker from '$lib/components/S3FilePicker.svelte';
type $$ComponentProps = {
    asset: Asset;
    _resourceMetadata?: {
        resource_type?: string;
    };
    s3FilePicker?: S3FilePicker;
    dbManagerDrawer?: DbManagerDrawer;
    onClick?: () => void;
    class?: string;
    noText?: boolean;
    buttonVariant?: ButtonType.Variant;
    btnClasses?: string;
};
declare const ExploreAssetButton: import("svelte").Component<$$ComponentProps, {}, "">;
type ExploreAssetButton = ReturnType<typeof ExploreAssetButton>;
export default ExploreAssetButton;
