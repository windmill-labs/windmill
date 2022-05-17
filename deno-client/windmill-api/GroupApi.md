# .GroupApi

All URIs are relative to */api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**addUserToGroup**](GroupApi.md#addUserToGroup) | **POST** /w/{workspace}/groups/adduser/{name} | add user to group
[**createGroup**](GroupApi.md#createGroup) | **POST** /w/{workspace}/groups/create | create group
[**deleteGroup**](GroupApi.md#deleteGroup) | **DELETE** /w/{workspace}/groups/delete/{name} | delete group
[**getGroup**](GroupApi.md#getGroup) | **GET** /w/{workspace}/groups/get/{name} | get group
[**listGroupNames**](GroupApi.md#listGroupNames) | **GET** /w/{workspace}/groups/listnames | list group names
[**listGroups**](GroupApi.md#listGroups) | **GET** /w/{workspace}/groups/list | list groups
[**removeUserToGroup**](GroupApi.md#removeUserToGroup) | **POST** /w/{workspace}/groups/removeuser/{name} | remove user to group
[**updateGroup**](GroupApi.md#updateGroup) | **POST** /w/{workspace}/groups/update/{name} | update group


# **addUserToGroup**
> string addUserToGroup(inlineObject18)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .GroupApi(configuration);

let body:.GroupApiAddUserToGroupRequest = {
  // string
  workspace: "workspace_example",
  // string
  name: "name_example",
  // InlineObject18
  inlineObject18: {
    username: "username_example",
  },
};

apiInstance.addUserToGroup(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **inlineObject18** | **InlineObject18**|  |
 **workspace** | [**string**] |  | defaults to undefined
 **name** | [**string**] |  | defaults to undefined


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
**200** | user added to group |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **createGroup**
> string createGroup(inlineObject16)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .GroupApi(configuration);

let body:.GroupApiCreateGroupRequest = {
  // string
  workspace: "workspace_example",
  // InlineObject16
  inlineObject16: {
    name: "name_example",
    summary: "summary_example",
  },
};

apiInstance.createGroup(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **inlineObject16** | **InlineObject16**|  |
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
**200** | group created |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **deleteGroup**
> string deleteGroup()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .GroupApi(configuration);

let body:.GroupApiDeleteGroupRequest = {
  // string
  workspace: "workspace_example",
  // string
  name: "name_example",
};

apiInstance.deleteGroup(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **name** | [**string**] |  | defaults to undefined


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
**200** | group deleted |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **getGroup**
> Group getGroup()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .GroupApi(configuration);

let body:.GroupApiGetGroupRequest = {
  // string
  workspace: "workspace_example",
  // string
  name: "name_example",
};

apiInstance.getGroup(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **name** | [**string**] |  | defaults to undefined


### Return type

**Group**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | group |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listGroupNames**
> Array<string> listGroupNames()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .GroupApi(configuration);

let body:.GroupApiListGroupNamesRequest = {
  // string
  workspace: "workspace_example",
};

apiInstance.listGroupNames(body).then((data:any) => {
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
**200** | group list |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listGroups**
> Array<Group> listGroups()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .GroupApi(configuration);

let body:.GroupApiListGroupsRequest = {
  // string
  workspace: "workspace_example",
  // number | which page to return (start at 1, default 1) (optional)
  page: 1,
  // number | number of items to return for a given page (default 30, max 100) (optional)
  perPage: 1,
};

apiInstance.listGroups(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **page** | [**number**] | which page to return (start at 1, default 1) | (optional) defaults to undefined
 **perPage** | [**number**] | number of items to return for a given page (default 30, max 100) | (optional) defaults to undefined


### Return type

**Array<Group>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | group list |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **removeUserToGroup**
> string removeUserToGroup(inlineObject19)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .GroupApi(configuration);

let body:.GroupApiRemoveUserToGroupRequest = {
  // string
  workspace: "workspace_example",
  // string
  name: "name_example",
  // InlineObject19
  inlineObject19: {
    username: "username_example",
  },
};

apiInstance.removeUserToGroup(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **inlineObject19** | **InlineObject19**|  |
 **workspace** | [**string**] |  | defaults to undefined
 **name** | [**string**] |  | defaults to undefined


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
**200** | user removed from group |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **updateGroup**
> string updateGroup(inlineObject17)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .GroupApi(configuration);

let body:.GroupApiUpdateGroupRequest = {
  // string
  workspace: "workspace_example",
  // string
  name: "name_example",
  // InlineObject17
  inlineObject17: {
    summary: "summary_example",
  },
};

apiInstance.updateGroup(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **inlineObject17** | **InlineObject17**|  |
 **workspace** | [**string**] |  | defaults to undefined
 **name** | [**string**] |  | defaults to undefined


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
**200** | group updated |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)


