# .VariableApi

All URIs are relative to */api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createVariable**](VariableApi.md#createVariable) | **POST** /w/{workspace}/variables/create | create variable
[**deleteVariable**](VariableApi.md#deleteVariable) | **DELETE** /w/{workspace}/variables/delete/{path} | delete variable
[**getVariable**](VariableApi.md#getVariable) | **GET** /w/{workspace}/variables/get/{path} | get variable
[**listContextualVariables**](VariableApi.md#listContextualVariables) | **GET** /w/{workspace}/variables/list_contextual | list contextual variables
[**listVariable**](VariableApi.md#listVariable) | **GET** /w/{workspace}/variables/list | list variables
[**updateVariable**](VariableApi.md#updateVariable) | **POST** /w/{workspace}/variables/update/{path} | update variable


# **createVariable**
> string createVariable(createVariable)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .VariableApi(configuration);

let body:.VariableApiCreateVariableRequest = {
  // string
  workspace: "workspace_example",
  // CreateVariable | new variable
  createVariable: {
    path: "path_example",
    value: "value_example",
    isSecret: true,
    description: "description_example",
  },
};

apiInstance.createVariable(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **createVariable** | **CreateVariable**| new variable |
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
**201** | variable created |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **deleteVariable**
> string deleteVariable()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .VariableApi(configuration);

let body:.VariableApiDeleteVariableRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
};

apiInstance.deleteVariable(body).then((data:any) => {
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
**200** | variable deleted |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **getVariable**
> ListableVariable getVariable()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .VariableApi(configuration);

let body:.VariableApiGetVariableRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
  // boolean | ask to decrypt secret if this variable is secret (if not secret no effect, default: true)  (optional)
  decryptSecret: true,
};

apiInstance.getVariable(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined
 **decryptSecret** | [**boolean**] | ask to decrypt secret if this variable is secret (if not secret no effect, default: true)  | (optional) defaults to undefined


### Return type

**ListableVariable**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | variable |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listContextualVariables**
> Array<ContextualVariable> listContextualVariables()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .VariableApi(configuration);

let body:.VariableApiListContextualVariablesRequest = {
  // string
  workspace: "workspace_example",
};

apiInstance.listContextualVariables(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined


### Return type

**Array<ContextualVariable>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | contextual variable list |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listVariable**
> Array<ListableVariable> listVariable()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .VariableApi(configuration);

let body:.VariableApiListVariableRequest = {
  // string
  workspace: "workspace_example",
};

apiInstance.listVariable(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined


### Return type

**Array<ListableVariable>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | variable list |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **updateVariable**
> string updateVariable(editVariable)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .VariableApi(configuration);

let body:.VariableApiUpdateVariableRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
  // EditVariable | updated variable
  editVariable: {
    path: "path_example",
    value: "value_example",
    isSecret: true,
    description: "description_example",
  },
};

apiInstance.updateVariable(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **editVariable** | **EditVariable**| updated variable |
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
**200** | variable updated |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)


