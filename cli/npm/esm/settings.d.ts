export interface SimplifiedSettings {
    auto_invite_enabled: boolean;
    auto_invite_as: string;
    auto_invite_mode: string;
    webhook?: string;
    deploy_to?: string;
    error_handler?: string;
    error_handler_extra_args?: any;
    error_handler_muted_on_cancel?: boolean;
    openai_resource_path?: string;
    code_completion_enabled: boolean;
    large_file_storage?: any;
    git_sync?: any;
    default_app?: string;
    default_scripts?: any;
    name: string;
}
export declare function pushWorkspaceSettings(workspace: string, _path: string, settings: SimplifiedSettings | undefined, localSettings: SimplifiedSettings): Promise<void>;
export declare function pushWorkspaceKey(workspace: string, _path: string, key: string | undefined, localKey: string): Promise<void>;
export declare function pullInstanceSettings(preview?: boolean): Promise<number | undefined>;
export declare function pushInstanceSettings(preview?: boolean, baseUrl?: string): Promise<number | undefined>;
export declare function pullInstanceConfigs(preview?: boolean): Promise<number | undefined>;
export declare function pushInstanceConfigs(preview?: boolean): Promise<number | undefined>;
//# sourceMappingURL=settings.d.ts.map