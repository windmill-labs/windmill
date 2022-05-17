# .GranularAclApi

All URIs are relative to */api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**addGranularAcls**](GranularAclApi.md#addGranularAcls) | **POST** /w/{workspace}/acls/add/{kind}/{path} | add granular acls
[**getGranularAcls**](GranularAclApi.md#getGranularAcls) | **GET** /w/{workspace}/acls/get/{kind}/{path} | get granular acls
[**removeGranularAcls**](GranularAclApi.md#removeGranularAcls) | **POST** /w/{workspace}/acls/remove/{kind}/{path} | remove granular acls


# **addGranularAcls**
> string addGranularAcls(inlineObject20)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .GranularAclApi(configuration);

let body:.GranularAclApiAddGranularAclsRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
  // 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow'
  kind: "script",
  // InlineObject20
  inlineObject20: {
    owner: "owner_example",
    write: true,
  },
};

apiInstance.addGranularAcls(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **inlineObject20** | **InlineObject20**|  |
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined
 **kind** | [**&#39;script&#39; | &#39;group_&#39; | &#39;resource&#39; | &#39;schedule&#39; | &#39;variable&#39; | &#39;flow&#39;**]**Array<&#39;script&#39; &#124; &#39;group_&#39; &#124; &#39;resource&#39; &#124; &#39;schedule&#39; &#124; &#39;variable&#39; &#124; &#39;flow&#39;>** |  | defaults to undefined


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
**200** | granular acl added |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **getGranularAcls**
> { [key: string]: boolean; } getGranularAcls()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .GranularAclApi(configuration);

let body:.GranularAclApiGetGranularAclsRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
  // 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow'
  kind: "script",
};

apiInstance.getGranularAcls(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined
 **kind** | [**&#39;script&#39; | &#39;group_&#39; | &#39;resource&#39; | &#39;schedule&#39; | &#39;variable&#39; | &#39;flow&#39;**]**Array<&#39;script&#39; &#124; &#39;group_&#39; &#124; &#39;resource&#39; &#124; &#39;schedule&#39; &#124; &#39;variable&#39; &#124; &#39;flow&#39;>** |  | defaults to undefined


### Return type

**{ [key: string]: boolean; }**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | acls |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **removeGranularAcls**
> string removeGranularAcls(inlineObject21)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .GranularAclApi(configuration);

let body:.GranularAclApiRemoveGranularAclsRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
  // 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow'
  kind: "script",
  // InlineObject21
  inlineObject21: {
    owner: "owner_example",
  },
};

apiInstance.removeGranularAcls(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **inlineObject21** | **InlineObject21**|  |
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined
 **kind** | [**&#39;script&#39; | &#39;group_&#39; | &#39;resource&#39; | &#39;schedule&#39; | &#39;variable&#39; | &#39;flow&#39;**]**Array<&#39;script&#39; &#124; &#39;group_&#39; &#124; &#39;resource&#39; &#124; &#39;schedule&#39; &#124; &#39;variable&#39; &#124; &#39;flow&#39;>** |  | defaults to undefined


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
**200** | granular acl removed |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)


