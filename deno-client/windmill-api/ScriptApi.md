# .ScriptApi

All URIs are relative to */api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**archiveScriptByHash**](ScriptApi.md#archiveScriptByHash) | **POST** /w/{workspace}/scripts/archive/h/{hash} | archive script by hash
[**archiveScriptByPath**](ScriptApi.md#archiveScriptByPath) | **POST** /w/{workspace}/scripts/archive/p/{path} | archive script by path
[**createScript**](ScriptApi.md#createScript) | **POST** /w/{workspace}/scripts/create | create script
[**deleteScriptByHash**](ScriptApi.md#deleteScriptByHash) | **POST** /w/{workspace}/scripts/delete/h/{hash} | delete script by hash (erase content but keep hash)
[**denoToJsonschema**](ScriptApi.md#denoToJsonschema) | **POST** /scripts/deno/tojsonschema | inspect deno code to infer jsonschema of arguments
[**getScriptByHash**](ScriptApi.md#getScriptByHash) | **GET** /w/{workspace}/scripts/get/h/{hash} | get script by hash
[**getScriptByPath**](ScriptApi.md#getScriptByPath) | **GET** /w/{workspace}/scripts/get/p/{path} | get script by path
[**getScriptDeploymentStatus**](ScriptApi.md#getScriptDeploymentStatus) | **GET** /w/{workspace}/scripts/deployment_status/h/{hash} | get script deployment status
[**listScripts**](ScriptApi.md#listScripts) | **GET** /w/{workspace}/scripts/list | list all available scripts
[**pythonToJsonschema**](ScriptApi.md#pythonToJsonschema) | **POST** /scripts/python/tojsonschema | inspect python code to infer jsonschema of arguments


# **archiveScriptByHash**
> Script archiveScriptByHash()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScriptApi(configuration);

let body:.ScriptApiArchiveScriptByHashRequest = {
  // string
  workspace: "workspace_example",
  // string
  hash: "hash_example",
};

apiInstance.archiveScriptByHash(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **hash** | [**string**] |  | defaults to undefined


### Return type

**Script**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | script details |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **archiveScriptByPath**
> string archiveScriptByPath()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScriptApi(configuration);

let body:.ScriptApiArchiveScriptByPathRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
};

apiInstance.archiveScriptByPath(body).then((data:any) => {
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
**200** | script archived |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **createScript**
> string createScript(inlineObject10)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScriptApi(configuration);

let body:.ScriptApiCreateScriptRequest = {
  // string
  workspace: "workspace_example",
  // InlineObject10
  inlineObject10: {
    path: "path_example",
    parentHash: "parentHash_example",
    summary: "summary_example",
    description: "description_example",
    content: "content_example",
    schema: {},
    isTemplate: true,
    lock: [
      "lock_example",
    ],
    language: "python3",
  },
};

apiInstance.createScript(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **inlineObject10** | **InlineObject10**|  |
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
**201** | script created |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **deleteScriptByHash**
> Script deleteScriptByHash()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScriptApi(configuration);

let body:.ScriptApiDeleteScriptByHashRequest = {
  // string
  workspace: "workspace_example",
  // string
  hash: "hash_example",
};

apiInstance.deleteScriptByHash(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **hash** | [**string**] |  | defaults to undefined


### Return type

**Script**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | script details |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **denoToJsonschema**
> MainArgSignature denoToJsonschema(body)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScriptApi(configuration);

let body:.ScriptApiDenoToJsonschemaRequest = {
  // string | deno code with the main function
  body: "body_example",
};

apiInstance.denoToJsonschema(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **body** | **string**| deno code with the main function |


### Return type

**MainArgSignature**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | parsed args |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **getScriptByHash**
> Script getScriptByHash()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScriptApi(configuration);

let body:.ScriptApiGetScriptByHashRequest = {
  // string
  workspace: "workspace_example",
  // string
  hash: "hash_example",
};

apiInstance.getScriptByHash(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **hash** | [**string**] |  | defaults to undefined


### Return type

**Script**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | script details |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **getScriptByPath**
> Script getScriptByPath()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScriptApi(configuration);

let body:.ScriptApiGetScriptByPathRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
};

apiInstance.getScriptByPath(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined


### Return type

**Script**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | script details |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **getScriptDeploymentStatus**
> InlineResponse2001 getScriptDeploymentStatus()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScriptApi(configuration);

let body:.ScriptApiGetScriptDeploymentStatusRequest = {
  // string
  workspace: "workspace_example",
  // string
  hash: "hash_example",
};

apiInstance.getScriptDeploymentStatus(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **hash** | [**string**] |  | defaults to undefined


### Return type

**InlineResponse2001**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | script details |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listScripts**
> Array<Script> listScripts()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScriptApi(configuration);

let body:.ScriptApiListScriptsRequest = {
  // string
  workspace: "workspace_example",
  // number | which page to return (start at 1, default 1) (optional)
  page: 1,
  // number | number of items to return for a given page (default 30, max 100) (optional)
  perPage: 1,
  // boolean | order by desc order (default true) (optional)
  orderDesc: true,
  // string | mask to filter exact matching user creator (optional)
  createdBy: "created_by_example",
  // string | mask to filter matching starting parh (optional)
  pathStart: "path_start_example",
  // string | mask to filter exact matching path (optional)
  pathExact: "path_exact_example",
  // string | mask to filter scripts whom first direct parent has exact hash (optional)
  firstParentHash: "first_parent_hash_example",
  // string | mask to filter scripts whom last parent in the chain has exact hash.  Beware that each script stores only a limited number of parents. Hence the last parent hash for a script is not necessarily its top-most parent. To find the top-most parent you will have to jump from last to last hash  until finding the parent  (optional)
  lastParentHash: "last_parent_hash_example",
  // string | is the hash present in the array of stored parent hashes for this script. The same warning applies than for last_parent_hash. A script only store a limited number of direct parent  (optional)
  parentHash: "parent_hash_example",
  // boolean | (default false) show also the archived files. when multiple archived hash share the same path, only the ones with the latest create_at are displayed.  (optional)
  showArchived: true,
  // boolean | (default regardless) if true show only the templates if false show only the non templates if not defined, show all regardless of if the script is a template  (optional)
  isTemplate: true,
};

apiInstance.listScripts(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **page** | [**number**] | which page to return (start at 1, default 1) | (optional) defaults to undefined
 **perPage** | [**number**] | number of items to return for a given page (default 30, max 100) | (optional) defaults to undefined
 **orderDesc** | [**boolean**] | order by desc order (default true) | (optional) defaults to undefined
 **createdBy** | [**string**] | mask to filter exact matching user creator | (optional) defaults to undefined
 **pathStart** | [**string**] | mask to filter matching starting parh | (optional) defaults to undefined
 **pathExact** | [**string**] | mask to filter exact matching path | (optional) defaults to undefined
 **firstParentHash** | [**string**] | mask to filter scripts whom first direct parent has exact hash | (optional) defaults to undefined
 **lastParentHash** | [**string**] | mask to filter scripts whom last parent in the chain has exact hash.  Beware that each script stores only a limited number of parents. Hence the last parent hash for a script is not necessarily its top-most parent. To find the top-most parent you will have to jump from last to last hash  until finding the parent  | (optional) defaults to undefined
 **parentHash** | [**string**] | is the hash present in the array of stored parent hashes for this script. The same warning applies than for last_parent_hash. A script only store a limited number of direct parent  | (optional) defaults to undefined
 **showArchived** | [**boolean**] | (default false) show also the archived files. when multiple archived hash share the same path, only the ones with the latest create_at are displayed.  | (optional) defaults to undefined
 **isTemplate** | [**boolean**] | (default regardless) if true show only the templates if false show only the non templates if not defined, show all regardless of if the script is a template  | (optional) defaults to undefined


### Return type

**Array<Script>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | All available scripts |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **pythonToJsonschema**
> MainArgSignature pythonToJsonschema(body)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScriptApi(configuration);

let body:.ScriptApiPythonToJsonschemaRequest = {
  // string | python code with the main function
  body: "body_example",
};

apiInstance.pythonToJsonschema(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **body** | **string**| python code with the main function |


### Return type

**MainArgSignature**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | parsed args |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)


