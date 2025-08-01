import { JSONSchema7 } from 'json-schema';
import { Dictionary } from './basic';
import { Extensions, IComponentNode, INode, INodeExample, INodeExternalExample, IShareableNode, ISpecExtensions } from './graph';
import { IServer } from './servers';
/**
 * HTTP Service
 */
export interface IHttpService extends INode, IShareableNode, ISpecExtensions {
    name: string;
    version: string;
    servers?: IServer[];
    security?: HttpSecurityScheme[][];
    securitySchemes?: HttpSecurityScheme[];
    termsOfService?: string;
    contact?: {
        name?: string;
        url?: string;
        email?: string;
    };
    license?: {
        name: string;
        url?: string;
        identifier?: string;
    };
    logo?: {
        altText: string;
        href?: string;
        url?: string;
        backgroundColor?: string;
    };
    infoExtensions?: Extensions;
    internal?: boolean;
    externalDocs?: IExternalDocs;
}
export interface IBundledHttpService extends Omit<IHttpService, 'securitySchemes'> {
    operations: IHttpOperation<true>[];
    webhooks: IHttpWebhookOperation<true>[];
    components: {
        schemas: (IComponentNode & JSONSchema7)[];
        responses: (IComponentNode & (IHttpOperationResponse<true> | Reference))[];
        path: (IComponentNode & (IHttpHeaderParam<true> | Reference))[];
        query: (IComponentNode & (IHttpQueryParam<true> | Reference))[];
        header: (IComponentNode & (IHttpHeaderParam<true> | Reference))[];
        cookie: (IComponentNode & (IHttpCookieParam<true> | Reference))[];
        /**
         * component parameters that are only references to external/unavailable
         * parameter definitions; parameters whose definitions are available
         * will always be found in path, query, header, or cookie.
         */
        unknownParameters: (IComponentNode & Reference)[];
        examples: (IComponentNode & (INodeExample | INodeExternalExample | Reference))[];
        requestBodies: (IComponentNode & (IHttpOperationRequestBody<true> | Reference))[];
        securitySchemes: (IComponentNode & (HttpSecurityScheme | Reference))[];
        callbacks: (IComponentNode & (IHttpCallbackOperation | IHttpKeyedReference))[];
    };
}
/**
 * HTTP Operation & Webhooks
 */
export interface IHttpEndpointOperation<Bundle extends boolean = false> extends INode, IShareableNode, ISpecExtensions {
    method: string;
    request?: Bundle extends true ? IHttpOperationRequest<true> | Reference : IHttpOperationRequest<false>;
    responses: (Bundle extends true ? IHttpOperationResponse<true> | (Pick<IHttpOperationResponse, 'code'> & Reference) : IHttpOperationResponse<false>)[];
    servers?: IServer[];
    callbacks?: (Bundle extends true ? (IHttpCallbackOperation | IHttpKeyedReference)[] : IHttpCallbackOperation<false>)[];
    security?: HttpSecurityScheme[][];
    securityDeclarationType?: HttpOperationSecurityDeclarationTypes;
    deprecated?: boolean;
    internal?: boolean;
    externalDocs?: IExternalDocs;
}
export interface IHttpOperation<Bundle extends boolean = false> extends IHttpEndpointOperation<Bundle> {
    path: string;
}
export interface IHttpWebhookOperation<Bundle extends boolean = false> extends IHttpEndpointOperation<Bundle> {
    name: string;
}
export declare enum HttpOperationSecurityDeclarationTypes {
    /** Indicates that the operation has no security declarations. */
    None = "none",
    /** Indicates that the operation has explicit security declarations. */
    Declared = "declared",
    /** Indicates that the operation inherits its security declarations from the service. */
    InheritedFromService = "inheritedFromService"
}
export interface IHttpCallbackOperation<Bundle extends boolean = false> extends Omit<IHttpOperation<Bundle>, 'callbacks'> {
    key: string;
}
export interface IHttpKeyedReference extends Reference {
    key: string;
}
export interface IHttpOperationRequest<Bundle extends boolean = false> {
    path?: (Bundle extends true ? IHttpPathParam<true> | Reference : IHttpPathParam<false>)[];
    query?: (Bundle extends true ? IHttpQueryParam<true> | Reference : IHttpQueryParam<false>)[];
    headers?: (Bundle extends true ? IHttpHeaderParam<true> | Reference : IHttpHeaderParam<false>)[];
    cookie?: (Bundle extends true ? IHttpCookieParam<true> | Reference : IHttpCookieParam<false>)[];
    unknown?: Reference[];
    body?: Bundle extends true ? IHttpOperationRequestBody<true> | Reference : IHttpOperationRequestBody<false>;
}
export interface IHttpOperationRequestBody<Bundle extends boolean = false> extends IShareableNode, ISpecExtensions {
    contents?: IMediaTypeContent<Bundle>[];
    required?: boolean;
    description?: string;
    name?: string;
}
export interface IHttpOperationResponse<Bundle extends boolean = false> extends IShareableNode, ISpecExtensions {
    code: string;
    contents?: IMediaTypeContent<Bundle>[];
    headers?: (Bundle extends true ? IHttpHeaderParam<true> | (Pick<IHttpHeaderParam, 'name'> & Reference) : IHttpHeaderParam<false>)[];
    description?: string;
}
/**
 * HTTP Params
 */
export interface IHttpParam<Bundle extends boolean = false> extends IHttpContent<Bundle>, IShareableNode, ISpecExtensions {
    name: string;
    style: HttpParamStyles;
    description?: string;
    explode?: boolean;
    required?: boolean;
    deprecated?: boolean;
    /** Captures any properties that were explicitly defined.  */
    explicitProperties?: string[];
}
export declare enum HttpParamStyles {
    /** Used when OAS2 type !== array */
    Unspecified = "unspecified",
    /**
     * OAS 3.x style simple
     * OAS 2 collectionFormat csv
     */
    Simple = "simple",
    /**
     * OAS 3.x style matrix
     * OAS 2 collectionFormat no support
     */
    Matrix = "matrix",
    /**
     * OAS 3.x style label
     * OAS 2 collectionFormat no support
     */
    Label = "label",
    /**
     * OAS 3.x style form
     * OAS 2 collectionFormat
     *   * csv, when explode === false
     *   * multi, when explode === true
     */
    Form = "form",
    /**
     * OAS 3.x no support
     * OAS 2 collectionFormat csv when explode === undefined
     */
    CommaDelimited = "commaDelimited",
    /**
     * OAS 3.x style spaceDelimited
     * OAS 2 collectionFormat ssv
     */
    SpaceDelimited = "spaceDelimited",
    /**
     * OAS 3.x style spaceDelimited
     * OAS 2 collectionFormat pipes
     */
    PipeDelimited = "pipeDelimited",
    /**
     * OAS 3.x style deepObject
     * OAS 2 collectionFormat no support
     */
    DeepObject = "deepObject",
    /**
     * OAS 3.x style no support
     * OAS 2 collectionFormat tsv
     */
    TabDelimited = "tabDelimited"
}
export interface IHttpPathParam<Bundle extends boolean = false> extends IHttpParam<Bundle> {
    style: HttpParamStyles.Unspecified | HttpParamStyles.Label | HttpParamStyles.Matrix | HttpParamStyles.Simple;
}
export interface IHttpQueryParam<Bundle extends boolean = false> extends IHttpParam<Bundle> {
    style: HttpParamStyles.Unspecified | HttpParamStyles.Form | HttpParamStyles.CommaDelimited | HttpParamStyles.SpaceDelimited | HttpParamStyles.PipeDelimited | HttpParamStyles.DeepObject | HttpParamStyles.TabDelimited;
    allowEmptyValue?: boolean;
    allowReserved?: boolean;
}
export interface IHttpHeaderParam<Bundle extends boolean = false> extends IHttpParam<Bundle> {
    style: HttpParamStyles.Unspecified | HttpParamStyles.Simple;
}
export interface IHttpCookieParam<Bundle extends boolean = false> extends IHttpParam<Bundle> {
    style: HttpParamStyles.Unspecified | HttpParamStyles.Form;
}
/**
 * HTTP Content
 */
export interface IHttpContent<Bundle extends boolean = false> extends IShareableNode, ISpecExtensions {
    schema?: JSONSchema7;
    examples?: (Bundle extends true ? INodeExample | INodeExternalExample | (IHttpKeyedReference) : INodeExample | INodeExternalExample)[];
    encodings?: IHttpEncoding<Bundle>[];
}
export interface IMediaTypeContent<Bundle extends boolean = false> extends IHttpContent<Bundle> {
    mediaType: string;
}
export interface IHttpEncoding<Bundle extends boolean = false> extends ISpecExtensions {
    property: string;
    style: HttpParamStyles.Form | HttpParamStyles.CommaDelimited | HttpParamStyles.SpaceDelimited | HttpParamStyles.PipeDelimited | HttpParamStyles.DeepObject;
    headers?: (Bundle extends true ? IHttpHeaderParam<true> | Reference : IHttpHeaderParam<false>)[];
    mediaType?: string;
    explode?: boolean;
    allowReserved?: boolean;
}
/**
 * HTTP Security
 */
export declare type HttpSecurityScheme = IApiKeySecurityScheme | IBearerSecurityScheme | IBasicSecurityScheme | IOauth2SecurityScheme | IOpenIdConnectSecurityScheme | IMutualTLSSecurityScheme;
interface ISecurityScheme extends IShareableNode, ISpecExtensions {
    key: string;
    description?: string;
}
export interface IApiKeySecurityScheme extends ISecurityScheme {
    type: 'apiKey';
    name: string;
    in: 'query' | 'header' | 'cookie';
}
export interface IBearerSecurityScheme extends ISecurityScheme {
    type: 'http';
    scheme: 'bearer';
    bearerFormat?: string;
}
export interface IBasicSecurityScheme extends ISecurityScheme {
    type: 'http';
    scheme: 'basic' | 'digest';
}
export interface IOpenIdConnectSecurityScheme extends ISecurityScheme {
    type: 'openIdConnect';
    openIdConnectUrl: string;
}
export interface IOauth2SecurityScheme extends ISecurityScheme {
    type: 'oauth2';
    flows: IOauthFlowObjects;
}
export interface IMutualTLSSecurityScheme extends ISecurityScheme {
    type: 'mutualTLS';
}
export interface IOauthFlowObjects {
    implicit?: IOauth2ImplicitFlow;
    password?: IOauth2PasswordFlow;
    clientCredentials?: IOauth2ClientCredentialsFlow;
    authorizationCode?: IOauth2AuthorizationCodeFlow;
}
export interface IOauth2Flow {
    scopes: Dictionary<string, string>;
    refreshUrl?: string;
}
export interface IOauth2ImplicitFlow extends IOauth2Flow {
    authorizationUrl: string;
}
export interface IOauth2AuthorizationCodeFlow extends IOauth2Flow {
    authorizationUrl: string;
    tokenUrl: string;
}
export interface IOauth2PasswordFlow extends IOauth2Flow {
    tokenUrl: string;
}
export interface IOauth2ClientCredentialsFlow extends IOauth2Flow {
    tokenUrl: string;
}
export declare type Reference = {
    $ref: string;
    summary?: string;
    description?: string;
};
export interface IExternalDocs extends ISpecExtensions {
    description?: string;
    url: string;
}
export {};
