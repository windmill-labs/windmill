# .WorkspaceApi

All URIs are relative to */api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createWorkspace**](WorkspaceApi.md#createWorkspace) | **POST** /workspaces/create | create workspace
[**deleteInvite**](WorkspaceApi.md#deleteInvite) | **POST** /w/{workspace}/workspaces/delete_invite | delete user invite
[**deleteWorkspace**](WorkspaceApi.md#deleteWorkspace) | **DELETE** /w/{workspace}/workspaces/delete | delete workspace
[**disconnectClient**](WorkspaceApi.md#disconnectClient) | **POST** /w/{workspace}/oauth/disconnect/{client_name} | disconnect client
[**editSlackCommand**](WorkspaceApi.md#editSlackCommand) | **POST** /w/{workspace}/workspaces/edit_slack_command | edit slack command
[**getSettings**](WorkspaceApi.md#getSettings) | **GET** /w/{workspace}/workspaces/get_settings | get settings
[**inviteUser**](WorkspaceApi.md#inviteUser) | **POST** /w/{workspace}/workspaces/invite_user | invite user to workspace
[**listPendingInvites**](WorkspaceApi.md#listPendingInvites) | **GET** /w/{workspace}/workspaces/list_pending_invites | list pending invites for a workspace
[**listUserWorkspaces**](WorkspaceApi.md#listUserWorkspaces) | **GET** /workspaces/users | list all workspaces visible to me with user info
[**listWorkspaces**](WorkspaceApi.md#listWorkspaces) | **GET** /workspaces/list | list all workspaces visible to me
[**listWorkspacesAsSuperAdmin**](WorkspaceApi.md#listWorkspacesAsSuperAdmin) | **GET** /workspaces/list_as_superadmin | list all workspaces as super admin (require to be super amdin)
[**validateId**](WorkspaceApi.md#validateId) | **POST** /workspaces/validate_id | validate id
[**validateUsername**](WorkspaceApi.md#validateUsername) | **POST** /workspaces/validate_username | validate username


# **createWorkspace**
> string createWorkspace(createWorkspace)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .WorkspaceApi(configuration);

let body:.WorkspaceApiCreateWorkspaceRequest = {
  // CreateWorkspace | new token
  createWorkspace: {
    id: "id_example",
    name: "name_example",
    username: "username_example",
    domain: "domain_example",
  },
};

apiInstance.createWorkspace(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **createWorkspace** | **CreateWorkspace**| new token |


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
**201** | token created |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **deleteInvite**
> string deleteInvite(inlineObject8)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .WorkspaceApi(configuration);

let body:.WorkspaceApiDeleteInviteRequest = {
  // string
  workspace: "workspace_example",
  // InlineObject8
  inlineObject8: {
    email: "email_example",
    isAdmin: true,
  },
};

apiInstance.deleteInvite(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **inlineObject8** | **InlineObject8**|  |
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
**200** | status |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **deleteWorkspace**
> string deleteWorkspace()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .WorkspaceApi(configuration);

let body:.WorkspaceApiDeleteWorkspaceRequest = {
  // string
  workspace: "workspace_example",
};

apiInstance.deleteWorkspace(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined


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
**200** | status |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **disconnectClient**
> string disconnectClient()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .WorkspaceApi(configuration);

let body:.WorkspaceApiDisconnectClientRequest = {
  // string
  workspace: "workspace_example",
  // string
  clientName: "client_name_example",
};

apiInstance.disconnectClient(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **clientName** | [**string**] |  | defaults to undefined


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
**200** | disconnected client |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **editSlackCommand**
> string editSlackCommand(inlineObject9)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .WorkspaceApi(configuration);

let body:.WorkspaceApiEditSlackCommandRequest = {
  // string
  workspace: "workspace_example",
  // InlineObject9
  inlineObject9: {
    slackCommandScript: "slackCommandScript_example",
  },
};

apiInstance.editSlackCommand(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **inlineObject9** | **InlineObject9**|  |
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
**200** | status |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **getSettings**
> InlineResponse200 getSettings()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .WorkspaceApi(configuration);

let body:.WorkspaceApiGetSettingsRequest = {
  // string
  workspace: "workspace_example",
};

apiInstance.getSettings(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined


### Return type

**InlineResponse200**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | status |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **inviteUser**
> string inviteUser(inlineObject7)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .WorkspaceApi(configuration);

let body:.WorkspaceApiInviteUserRequest = {
  // string
  workspace: "workspace_example",
  // InlineObject7
  inlineObject7: {
    email: "email_example",
    isAdmin: true,
  },
};

apiInstance.inviteUser(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **inlineObject7** | **InlineObject7**|  |
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
**200** | status |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listPendingInvites**
> Array<WorkspaceInvite> listPendingInvites()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .WorkspaceApi(configuration);

let body:.WorkspaceApiListPendingInvitesRequest = {
  // string
  workspace: "workspace_example",
};

apiInstance.listPendingInvites(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined


### Return type

**Array<WorkspaceInvite>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | user |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listUserWorkspaces**
> UserWorkspaceList listUserWorkspaces()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .WorkspaceApi(configuration);

let body:any = {};

apiInstance.listUserWorkspaces(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters
This endpoint does not need any parameter.


### Return type

**UserWorkspaceList**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | workspace with associated username |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listWorkspaces**
> Array<Workspace> listWorkspaces()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .WorkspaceApi(configuration);

let body:any = {};

apiInstance.listWorkspaces(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters
This endpoint does not need any parameter.


### Return type

**Array<Workspace>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | all workspaces |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listWorkspacesAsSuperAdmin**
> Array<Workspace> listWorkspacesAsSuperAdmin()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .WorkspaceApi(configuration);

let body:.WorkspaceApiListWorkspacesAsSuperAdminRequest = {
  // number | which page to return (start at 1, default 1) (optional)
  page: 1,
  // number | number of items to return for a given page (default 30, max 100) (optional)
  perPage: 1,
};

apiInstance.listWorkspacesAsSuperAdmin(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **page** | [**number**] | which page to return (start at 1, default 1) | (optional) defaults to undefined
 **perPage** | [**number**] | number of items to return for a given page (default 30, max 100) | (optional) defaults to undefined


### Return type

**Array<Workspace>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | workspaces |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **validateId**
> string validateId(inlineObject3)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .WorkspaceApi(configuration);

let body:.WorkspaceApiValidateIdRequest = {
  // InlineObject3
  inlineObject3: {
    id: "id_example",
  },
};

apiInstance.validateId(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **inlineObject3** | **InlineObject3**|  |


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
**200** | status |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **validateUsername**
> string validateUsername(inlineObject4)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .WorkspaceApi(configuration);

let body:.WorkspaceApiValidateUsernameRequest = {
  // InlineObject4
  inlineObject4: {
    id: "id_example",
    username: "username_example",
  },
};

apiInstance.validateUsername(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **inlineObject4** | **InlineObject4**|  |


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
**200** | status |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)


