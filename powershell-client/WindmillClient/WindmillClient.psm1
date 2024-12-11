$script:WindmillConnection = $null

<#
.SYNOPSIS
Connects to Windmill
#>
function Connect-Windmill {
    param(
        [string] $BaseUrl = $null,
        [string] $Token = $null, 
        [string] $Workspace = $null
    )

    $script:WindmillConnection = [Windmill]::new($BaseUrl, $Token, $Workspace)
}

<#
.SYNOPSIS
Disconnects from Windmill
#>
function Disconnect-Windmill {
    $script:WindmillConnection = $null
}

<#
.SYNOPSIS
Creates a new token
#>
function New-WindmillToken() {
    param(
        [TimeSpan] $Duration = (New-TimeSpan -Days 1)
    )
    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $script:WindmillConnection.CreateToken([DateTime]::Now.Add($Duration))
}

<#
.SYNOPSIS
Returns OIDC token for specified audience
#>
function Get-WindmillIdToken {
    param(
        [string] $Audience
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $script:WindmillConnection.GetIdToken($Audience)
}

<#
.SYNOPSIS
Returns current sorkspace
#>
function Get-WindmillWorkspace {
    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $script:WindmillConnection.Workspace
}

<#
.SYNOPSIS
Returns Windmill version
#>
function Get-WindmillVersion {
    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $script:WindmillConnection.Version()
}

<#
.SYNOPSIS
Returns current user
#>

function Get-WindmillUser {
    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $script:WindmillConnection.Whoami()
}

<#
.SYNOPSIS
Returns the value of specified variable
#>
function Get-WindmillVariable {
    param(
        [string] $Path
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $script:WindmillConnection.GetVariable($Path)
}

<#
.SYNOPSIS Creates a new variable with specified value
#>
function New-WindmillVariable {
    param(
        [string] $Path,
        [string] $Value,
        [switch] $Secret
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    $script:WindmillConnection.CreateVariable($Path, $Value, $Secret)
}

<#
.SYNOPSIS
Sets the value of specified variable
#>
function Set-WindmillVariable {
    param(
        [string] $Path,
        [string] $Value,
        [switch] $Secret
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    $script:WindmillConnection.SetVariable($Path, $Value, $Secret)
}

<#
.SYNOPSIS Deletes a variable
#>
function Remove-WindmillVariable {
    param(
        [string] $Path
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    $script:WindmillConnection.DeleteVariable($Path)
}

<#
.SYNOPSIS
Returns the value of specified resource
#>
function Get-WindmillResource {
    param(
        [string] $Path
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $script:WindmillConnection.GetResource($Path)
}

<#
.SYNOPSIS
Creates a new resource with specified value
#>
function New-WindmillResource {
    param(
        [string] $Path,
        [Hashtable] $Value,
        [string] $ResourceType = $null
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    $script:WindmillConnection.CreateResource($Path, $Value, $ResourceType)
}

<#
.SYNOPSIS
Sets the value of specified resource
#>
function Set-WindmillResource {
    param(
        [string] $Path = $null,
        [Hashtable] $Value,
        [string] $ResourceType = $null
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }
    $script:WindmillConnection.SetResource($Path, $Value, $ResourceType)
}

<#
.SYNOPSIS
Sets the value of a resource with type "state".
#>
function Set-WindmillState {
    param(
        [Hashtable] $Value
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    $script:WindmillConnection.SetResource($script:WindmillConnection.GetStatePath(), $Value, "state")
}

<#
.SYNOPSIS
Gets the value of a resource with type "state".
#>
function Get-WindmillState {
    Get-WindmillResource -Path $script:WindmillConnection.GetStatePath()
}

<#
.SYNOPSIS
Deletes a resource
#>
function Remove-WindmillResource {
    param(
        [string] $Path
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    $script:WindmillConnection.DeleteResource($Path)
}

<#
.SYNOPSIS
Synchronously runs a script
#>
function Invoke-WindmillScript {
    # Runs job and waits for it to complete
    param(
        [string] $Path = $null,
        [string] $Hash = $null,
        [Hashtable] $Arguments = @{},
        [boolean] $AssertResultIsNotNull = $true,
        [int] $Timeout = $null
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    $jobId = Start-WindmillScript -Path $Path -Hash $Hash -Arguments $Arguments
    $until = if ($Timeout) { (Get-Date).AddSeconds($Timeout) } else { [DateTime]::MaxValue }
    return $script:WindmillConnection.WaitJob($jobId, $until, $AssertResultIsNotNull)
}

<#
.SYNOPSIS
Asynchronously runs a script
#>
function Start-WindmillScript {
    param(
        [string] $Path = $null,
        [string] $Hash = $null,
        [Hashtable] $Arguments = @{},
        [int] $ScheduledInSecs = $null
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $script:WindmillConnection.RunScriptAsync($Path, $Hash, $Arguments, $ScheduledInSecs)
}

<#
.SYNOPSIS
Asynchronously runs a flow
#>
function Start-WindmillFlow {
    param(
        [string] $Path = $null,
        [Hashtable] $Arguments = @{},
        [int] $ScheduledInSecs = $null
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $script:WindmillConnection.RunFlowAsync($Path, $Arguments, $ScheduledInSecs)
}

<#
.SYNOPSIS
Stops a job
#>
function Stop-WindmillJob {
    param(
        [string] $JobId,
        [string] $Reason = ""
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $script:WindmillConnection.CancelJob($JobId, $Reason)
}

<#
.SYNOPSIS
Wait for a job to complete
#>
function Wait-WindmillJob {
    param(
        [string] $JobId,
        [timespan] $Timeout = [timespan]::MaxValue,
        [boolean] $AssertResultIsNotNull = $true
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $script:WindmillConnection.WaitJob($JobId, (Get-Date).Add($Timeout), $AssertResultIsNotNull)
}

<#
.SYNOPSIS
Stops all running executions of the same script
#>
function Stop-WindmillExecution {
    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $script:WindmillConnection.StopExecution()
}

<#
.SYNOPSIS
Returns a specified job
#>
function Get-WindmillJob {
    param(
        [string] $JobId = $null
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    if (-not $JobId) {
        return $script:WindmillConnection.ListJobs()
    }
    return $script:WindmillConnection.GetJob($JobId)
}

<#
.SYNOPSIS
Returns the result of a specified job
#>
function Get-WindmillResult {
    param(
        [string] $JobId,
        [switch] $AssertResultIsNotNull
    )

    if (-not $script:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $script:WindmillConnection.GetResult($JobId, $AssertResultIsNotNull)
}

class Windmill {
    [string] $BaseUrl
    [string] $Token
    [string] $Workspace
    [Hashtable] $Headers
    [string] $Path

    Windmill(
        [string] $BaseUrl = $null,
        [string] $Token = $null, 
        [string] $Workspace = $null
    ) {
        $this.BaseUrl = if ($BaseUrl) { $BaseUrl } else { $env:BASE_INTERNAL_URL }
        $this.BaseUrl = "$($this.BaseUrl)/api" 
        $this.Token = if ($Token) { $Token } else { $env:WM_TOKEN }
        $this.Headers = @{
            "Content-Type"  = "application/json"
            "Authorization" = "Bearer $($this.Token)"
        }
        $this.Workspace = if ($Workspace) { $Workspace } else { $env:WM_WORKSPACE } 
        if (-not $this.Workspace) {
            throw "Workspace required as an argument or WM_WORKSPACE environment variable"
        }

        $this.Path = $env:WM_JOB_PATH
    }

    [String] AddQueryParams([String] $Endpoint, [Hashtable] $QueryParams) {
        $url = $Endpoint

        if ($QueryParams.Count -gt 0) {
            $url += '?'

            $QueryParams.GetEnumerator() | ForEach-Object {
                $url += "$($_.Key)=$($_.Value)&"
            }

            # Remove the trailing '&'
            $url = $url.TrimEnd('&')
        }

        return $url
    }

    [Microsoft.PowerShell.Commands.BasicHtmlWebResponseObject] Get([string] $Endpoint, [boolean] $RaiseForStatus) {
        $Url = "$($this.BaseUrl)/$($Endpoint.TrimStart('/'))"
        $Response = Invoke-WebRequest -Uri $Url -Method "GET" -Headers $this.Headers -SkipHttpErrorCheck
        
        if ($RaiseForStatus -and -not $Response.BaseResponse.IsSuccessStatusCode) {
            throw "Request failed with status code $($Response.StatusCode)"
        }

        return $Response
    }

    [Microsoft.PowerShell.Commands.BasicHtmlWebResponseObject] Post([string] $Endpoint, [Object] $Data, [boolean] $RaiseForStatus) {
        $Url = "$($this.BaseUrl)/$($Endpoint.TrimStart('/'))"
        $Response = Invoke-WebRequest -Uri $Url -Method "POST" -Headers $this.Headers -Body ($Data | ConvertTo-Json) -SkipHttpErrorCheck -ContentType "application/json"
        if ($RaiseForStatus -and -not $Response.BaseResponse.IsSuccessStatusCode) {
            throw "Request failed with status code $($Response.StatusCode)"
        }

        return $Response
    }

    [Microsoft.PowerShell.Commands.BasicHtmlWebResponseObject] Delete([string] $Endpoint, [boolean] $RaiseForStatus) {
        $Url = "$($this.BaseUrl)/$($Endpoint.TrimStart('/'))"
        $Response = Invoke-WebRequest -Uri $Url -Method "DELETE" -Headers $this.Headers -SkipHttpErrorCheck
        
        if ($RaiseForStatus -and -not $Response.BaseResponse.IsSuccessStatusCode) {
            throw "Request failed with status code $($Response.StatusCode)"
        }

        return $Response
    }

    [string] Version() {
        $response = $this.Get("/version", $true)
        return $response.Content
    }
    
    [PSCustomObject] Whoami() {
        $response = $this.Get("/users/whoami", $true)
        $result = $response.Content | ConvertFrom-Json
        return $result
    }

    [string] CreateToken([datetime] $Expiration) {
        $endpoint = "users/tokens/create"
        $refresh = Get-Date (Get-Date).ToUniversalTime() -UFormat %s
        $payload = @{
            "label"      = "refresh $refresh"
            "expiration" = $Expiration.ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
        }

        return $this.Post($endpoint, $payload, $true).Content
    }

    [string] GetIdToken([string] $Audience) {
        return $this.Post("/w/$($this.Workspace)/oidc/token/$Audience").Content
    }

    [string] GetVariable([string] $Path) {
        $response = $this.Get("/w/$($this.Workspace)/variables/get_value/$Path", $true)
        return $response.Content | ConvertFrom-Json
    }

    [void] CreateVariable([string] $Path, [string] $Value, [boolean] $Secret) {
        $this.Post("/w/$($this.Workspace)/variables/create", @{ "path" = $Path; "value" = $Value; "is_secret" = $Secret; "description" = "" }, $true)
    }

    [void] SetVariable([string] $Path, [string] $Value, [boolean] $Secret) {
        $response = $this.Get("/w/$($this.Workspace)/variables/get_value/$Path", $true)

        if ($response.StatusCode -eq 404) {
            throw "Variable $Path not found"
        }
        else {
            $this.Post("/w/$($this.Workspace)/variables/update/$Path", @{ "value" = $Value }, $true)
        }
    }

    [void] DeleteVariable([string] $Path) {
        $this.Delete("/w/$($this.Workspace)/variables/delete/$Path", $true)
    }

    [void] CreateResource([string] $Path, [Hashtable] $Value, [string] $ResourceType) {
        $this.Post("/w/$($this.Workspace)/resources/create", @{ "path" = $Path; "value" = $Value; "resource_type" = $ResourceType }, $true)
    }

    [PSCustomObject] GetResource([string] $Path) {
        $response = $this.Get("/w/$($this.Workspace)/resources/get/$Path", $true)
        return $response.Content | ConvertFrom-Json
    }

    [void] SetResource([string] $Path, [Hashtable] $Value, [string] $ResourceType) {    
        # Resolve the effective path
        $resolvedPath = if ($Path) { $Path } else { $script:WindmillConnection.GetStatePath() }
    
        if ($this.Get("/w/$($this.Workspace)/resources/exists/$resolvedPath", $false).Content -eq "true") {
            $this.Post("/w/$($this.Workspace)/resources/update_value/$resolvedPath", @{ "value" = $Value }, $true)
        }

        elseif ($ResourceType) {
            $this.CreateResource($resolvedPath, $Value, $ResourceType)
        }

        else {
            throw "Resource at path $resolvedPath does not exist and no type was provided to initialize it"
        }
    }

    [void] DeleteResource([string] $Path) {
        $this.Delete("/w/$($this.Workspace)/resources/delete/$Path", $true)
    }

    [PSCustomObject] GetJob([string] $JobId) {
        $response = $this.Get("/w/$($this.Workspace)/jobs_u/get/$JobId", $true)
        return $response.Content | ConvertFrom-Json
    }

    [PSCustomObject] WaitJob([string] $JobId, [datetime] $Until, $AssertResultIsNotNull) {
        # TODO: Add cleanup
        while ((Get-Date) -lt $Until) {
            $response = $this.Get("/w/$($this.Workspace)/jobs_u/completed/get_result_maybe/$JobId", $false)
            $job = $response.Content | ConvertFrom-Json
            if ($job.completed) {
                if ($job.success) {
                    if ($AssertResultIsNotNull -and -not $job.result) {
                        throw "result is null for job $JobId"
                    }
                    
                    return $job.result
                }
                else {
                    $err = $job.result.error
                    throw "Job $JobId failed with error: $err"
                }

            }

            Start-Sleep -Milliseconds 500
        }

        throw "Job $JobId did not complete before $Until"
    }

    [PSCustomObject[]] ListJobs() {
        $response = $this.Get("/w/$($this.Workspace)/jobs/list", $true)
        return $response.Content | ConvertFrom-Json
    }

    [string] CancelJob([string] $JobId, [string] $Reason) {
        return $this.Post("/w/$($this.Workspace)/jobs_u/queue/cancel/$JobId", @{ "reason" = $Reason }, $true)
    }

    [PSCustomObject] GetResult([string] $JobId, [boolean] $AssertResultIsNotNull) {
        $response = $this.Get("/w/$($this.Workspace)/jobs_u/completed/get_result/$JobId", $true)
        $result = $response.Content | ConvertFrom-Json
        if ($AssertResultIsNotNull -and -not $result) {
            throw "result is null for job $JobId"
        }
        
        return $result
    }

    [PSCustomObject] RunScriptAsync([string] $Path, [string] $Hash, [Hashtable] $Arguments, [int] $ScheduledInSecs) {
        $params = @{}

        if ($Path -and $Hash) {
            throw "Path and Hash are mutually exclusive"
        }

        if ($ScheduledInSecs -ne $null) {
            $params["scheduled_in_secs"] = $ScheduledInSecs
        }

        if ($env:WM_JOB_ID) {
            $params["parent_job"] = $env:WM_JOB_ID
        }
        if ($env:WM_ROOT_FLOW_JOB_ID) {
            $params["root_job"] = $env:WM_ROOT_FLOW_JOB_ID
        }

        if ($Path) {
            $endpoint = "/w/$($this.Workspace)/jobs/run/p/$Path"
        }
        elseif ($Hash) {
            $endpoint = "/w/$($this.Workspace)/jobs/run/h/$Hash"
        }
        else {
            throw "Path or Hash must be provided"
        }

        if ($params) {
            $endpoint = $this.AddQueryParams($endpoint, $params)
        }

        return $this.Post($endpoint, $Arguments, $true).Content
    }

    [string] RunFlowAsync([string] $Path, [Hashtable] $Arguments, [int] $ScheduledInSecs) {
        $params = @{}

        if ($ScheduledInSecs -ne $null) {
            $params["scheduled_in_secs"] = $ScheduledInSecs
        }

        # TODO: Figure out why this fails when we set parent_job (at least for HN Discord Feed)
        if ($env:WM_JOB_ID) {
            $params["parent_job"] = $env:WM_JOB_ID
        }
        if ($env:WM_ROOT_FLOW_JOB_ID) {
            $params["root_job"] = $env:WM_ROOT_FLOW_JOB_ID
        }

        $endpoint = "/w/$($this.Workspace)/jobs/run/f/$Path"

        if ($params) {
            $endpoint = $this.AddQueryParams($endpoint, $params)
        }

        return $this.Post($endpoint, $Arguments, $true).Content
    }

    [Hashtable] StopExecution() {
        $params = @{
            "running"           = "true"
            "script_path_exact" = $this.Path
        }
        $endpoint = $this.AddQueryParams("/w/$($this.Workspace)/jobs/list", $params)
        $jobs = $this.Get($endpoint, $true).Content | ConvertFrom-Json
        $current_job_id = $env:WM_JOB_ID

        $job_ids = $jobs | Where-Object { $_.id -ne $current_job_id } | Select-Object -ExpandProperty id

        $result = @{}

        foreach ($job_id in $job_ids) {
            $result[$job_id] = $this.CancelJob($job_id, "Killed by Stop-WindmillExecution")
        }

        return $result
    }

    [string] GetStatePath() {
        $statePath = $env:WM_STATE_PATH_NEW

        if (-not $statePath) {
            $statePath = $env:WM_STATE_PATH
        }

        if (-not $statePath) {
            throw "State path not set"
        }
    
        return $statePath
    }
}

Export-ModuleMember -Function @(
    'Connect-Windmill',
    'Disconnect-Windmill',
    'New-WindmillToken',
    'Get-WindmillIdToken',
    'Get-WindmillWorkspace',
    'Get-WindmillVersion',
    'Get-WindmillUser',
    'Get-WindmillVariable',
    'New-WindmillVariable',
    'Set-WindmillVariable',
    'Remove-WindmillVariable',
    'Get-WindmillResource',
    'Get-WindmillState',
    'New-WindmillResource',
    'Set-WindmillResource',
    'Set-WindmillState',
    'Remove-WindmillResource',
    'Invoke-WindmillScript',
    'Start-WindmillScript',
    'Start-WindmillFlow',
    'Stop-WindmillJob',
    'Wait-WindmillJob',
    'Stop-WindmillExecution',
    'Get-WindmillJob',
    'Get-WindmillResult'
)