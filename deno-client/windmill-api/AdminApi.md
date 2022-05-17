# .AdminApi

All URIs are relative to */api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createUser**](AdminApi.md#createUser) | **POST** /w/{workspace}/users/add | create user (require admin privilege)
[**deleteUser**](AdminApi.md#deleteUser) | **DELETE** /w/{workspace}/users/delete/{username} | delete user (require admin privilege)
[**updateUser**](AdminApi.md#updateUser) | **POST** /w/{workspace}/users/update/{username} | update user (require admin privilege)


# **createUser**
> string createUser(newUser)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .AdminApi(configuration);

let body:.AdminApiCreateUserRequest = {
  // string
  workspace: "workspace_example",
  // NewUser | new user
  newUser: {
    email: "email_example",
    username: "username_example",
    isAdmin: true,
  },
};

apiInstance.createUser(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **newUser** | **NewUser**| new user |
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
**201** | user created |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **deleteUser**
> string deleteUser()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .AdminApi(configuration);

let body:.AdminApiDeleteUserRequest = {
  // string
  workspace: "workspace_example",
  // string
  username: "username_example",
};

apiInstance.deleteUser(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **username** | [**string**] |  | defaults to undefined


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
**200** | delete user |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **updateUser**
> string updateUser(editWorkspaceUser)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .AdminApi(configuration);

let body:.AdminApiUpdateUserRequest = {
  // string
  workspace: "workspace_example",
  // string
  username: "username_example",
  // EditWorkspaceUser | new user
  editWorkspaceUser: {
    isAdmin: true,
  },
};

apiInstance.updateUser(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **editWorkspaceUser** | **EditWorkspaceUser**| new user |
 **workspace** | [**string**] |  | defaults to undefined
 **username** | [**string**] |  | defaults to undefined


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
**200** | edited user |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)


