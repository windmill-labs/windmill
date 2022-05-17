# .ResourceApi

All URIs are relative to */api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createResource**](ResourceApi.md#createResource) | **POST** /w/{workspace}/resources/create | create resource
[**createResourceType**](ResourceApi.md#createResourceType) | **POST** /w/{workspace}/resources/type/create | create resource_type
[**deleteResource**](ResourceApi.md#deleteResource) | **DELETE** /w/{workspace}/resources/delete/{path} | delete resource
[**deleteResourceType**](ResourceApi.md#deleteResourceType) | **DELETE** /w/{workspace}/resources/type/delete/{path} | delete resource_type
[**getResource**](ResourceApi.md#getResource) | **GET** /w/{workspace}/resources/get/{path} | get resource
[**getResourceType**](ResourceApi.md#getResourceType) | **GET** /w/{workspace}/resources/type/get/{path} | get resource_type
[**listResource**](ResourceApi.md#listResource) | **GET** /w/{workspace}/resources/list | list resources
[**listResourceType**](ResourceApi.md#listResourceType) | **GET** /w/{workspace}/resources/type/list | list resource_types
[**listResourceTypeNames**](ResourceApi.md#listResourceTypeNames) | **GET** /w/{workspace}/resources/type/listnames | list resource_types names
[**updateResource**](ResourceApi.md#updateResource) | **POST** /w/{workspace}/resources/update/{path} | update resource
[**updateResourceType**](ResourceApi.md#updateResourceType) | **POST** /w/{workspace}/resources/type/update/{path} | update resource_type


# **createResource**
> string createResource(createResource)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ResourceApi(configuration);

let body:.ResourceApiCreateResourceRequest = {
  // string
  workspace: "workspace_example",
  // CreateResource | new resource
  createResource: {
    path: "path_example",
    value: {},
    description: "description_example",
    resourceType: "resourceType_example",
  },
};

apiInstance.createResource(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **createResource** | **CreateResource**| new resource |
 **workspace** | [**string**] |  | defaults to undefined


### Return type

**string**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/plain


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | resource created |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **createResourceType**
> string createResourceType(resourceType)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ResourceApi(configuration);

let body:.ResourceApiCreateResourceTypeRequest = {
  // string
  workspace: "workspace_example",
  // ResourceType | new resource_type
  resourceType: {
    workspaceId: "workspaceId_example",
    name: "name_example",
    schema: null,
    description: "description_example",
  },
};

apiInstance.createResourceType(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **resourceType** | **ResourceType**| new resource_type |
 **workspace** | [**string**] |  | defaults to undefined


### Return type

**string**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/plain


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | resource_type created |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **deleteResource**
> string deleteResource()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ResourceApi(configuration);

let body:.ResourceApiDeleteResourceRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
};

apiInstance.deleteResource(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined


### Return type

**string**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: text/plain


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | resource deleted |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **deleteResourceType**
> string deleteResourceType()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ResourceApi(configuration);

let body:.ResourceApiDeleteResourceTypeRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
};

apiInstance.deleteResourceType(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined


### Return type

**string**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: text/plain


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | resource_type deleted |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **getResource**
> Resource getResource()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ResourceApi(configuration);

let body:.ResourceApiGetResourceRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
};

apiInstance.getResource(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined


### Return type

**Resource**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | resource deleted |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **getResourceType**
> ResourceType getResourceType()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ResourceApi(configuration);

let body:.ResourceApiGetResourceTypeRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
};

apiInstance.getResourceType(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined


### Return type

**ResourceType**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | resource_type deleted |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listResource**
> Array<Resource> listResource()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ResourceApi(configuration);

let body:.ResourceApiListResourceRequest = {
  // string
  workspace: "workspace_example",
  // number | which page to return (start at 1, default 1) (optional)
  page: 1,
  // number | number of items to return for a given page (default 30, max 100) (optional)
  perPage: 1,
  // string | resource_type to list from (optional)
  resourceType: "resource_type_example",
};

apiInstance.listResource(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **page** | [**number**] | which page to return (start at 1, default 1) | (optional) defaults to undefined
 **perPage** | [**number**] | number of items to return for a given page (default 30, max 100) | (optional) defaults to undefined
 **resourceType** | [**string**] | resource_type to list from | (optional) defaults to undefined


### Return type

**Array<Resource>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | resource list |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listResourceType**
> Array<ResourceType> listResourceType()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ResourceApi(configuration);

let body:.ResourceApiListResourceTypeRequest = {
  // string
  workspace: "workspace_example",
};

apiInstance.listResourceType(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined


### Return type

**Array<ResourceType>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | resource_type list |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listResourceTypeNames**
> Array<string> listResourceTypeNames()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ResourceApi(configuration);

let body:.ResourceApiListResourceTypeNamesRequest = {
  // string
  workspace: "workspace_example",
};

apiInstance.listResourceTypeNames(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined


### Return type

**Array<string>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | resource_type list |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **updateResource**
> string updateResource(editResource)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ResourceApi(configuration);

let body:.ResourceApiUpdateResourceRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
  // EditResource | updated resource
  editResource: {
    path: "path_example",
    description: "description_example",
    value: {},
  },
};

apiInstance.updateResource(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **editResource** | **EditResource**| updated resource |
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined


### Return type

**string**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/plain


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | resource updated |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **updateResourceType**
> string updateResourceType(editResourceType)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ResourceApi(configuration);

let body:.ResourceApiUpdateResourceTypeRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
  // EditResourceType | updated resource_type
  editResourceType: {
    schema: "schema_example",
    description: "description_example",
  },
};

apiInstance.updateResourceType(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **editResourceType** | **EditResourceType**| updated resource_type |
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined


### Return type

**string**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/plain


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | resource_type updated |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)


