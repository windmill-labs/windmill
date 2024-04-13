$global:WindmillConnection = $null

function Connect-Windmill {
    param(
        [string] $BaseUrl = $null,
        [string] $Token = $null, 
        [string] $Workspace = $null
    )

    $global:WindmillConnection = [Windmill]::new($BaseUrl, $Token, $Workspace)
}

function Get-WindmillWorkspace {
    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $global:WindmillConnection.Workspace
}

function Get-WindmillVariable {
    param(
        [string] $Path
    )

    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $global:WindmillConnection.GetVariable($Path)
}

function Set-WindmillVariable {
    param(
        [string] $Path,
        [string] $Value,
        [switch] $Secret
    )

    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    $global:WindmillConnection.SetVariable($Path, $Value, $Secret)
}

function Get-WindmillResource {
    param(
        [string] $Path
    )

    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $global:WindmillConnection.GetResource($Path)
}

function Set-WindmillResource {
    param(
        [string] $Path,
        [Hashtable] $Value,
        [string] $ResourceType = $null
    )

    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    $global:WindmillConnection.SetResource($Path, $Value, $ResourceType)
}

function Get-WindmillResult {
    param(
        [string] $JobId,
        [switch] $AssertResultIsNotNull
    )

    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $global:WindmillConnection.GetResult($JobId, $AssertResultIsNotNull)
}

function Get-WindmillVersion {
    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $global:WindmillConnection.Version()
}

function Get-WindmillWorkspace {
    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $global:WindmillConnection.Workspace
}

function Get-WindmillUser {
    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $global:WindmillConnection.Whoami()
}

function Start-WindmillScript {
    param(
        [string] $Path = $null,
        [string] $Hash = $null,
        [Hashtable] $Arguments = @{},
        [int] $ScheduledInSecs = $null
    )

    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $global:WindmillConnection.RunScriptAsync($Path, $Hash, $Arguments, $ScheduledInSecs)
}

function Start-WindmillFlow {
    param(
        [string] $Path = $null,
        [Hashtable] $Arguments = @{},
        [int] $ScheduledInSecs = $null
    )

    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $global:WindmillConnection.RunFlowAsync($Path, $Arguments, $ScheduledInSecs)
}

function Invoke-WindmillScript {
    # Runs job and waits for it to complete
    param(
        [string] $Path = $null,
        [string] $Hash = $null,
        [Hashtable] $Arguments = @{},
        [boolean] $AssertResultIsNotNull = $true,
        [int] $Timeout = $null
    )

    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    $jobId = Start-WindmillScript -Path $Path -Hash $Hash -Arguments $Arguments
    $until = if ($Timeout) { (Get-Date).AddSeconds($Timeout) } else { [DateTime]::MaxValue }
    return $global:WindmillConnection.WaitJob($jobId, $until, $AssertResultIsNotNull)
}

function Stop-WindmillExecution {
    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $global:WindmillConnection.StopExecution()
}

function Get-WindmillJob {
    param(
        [string] $JobId
    )

    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $global:WindmillConnection.GetJob($JobId)
}

function Stop-WindmillJob {
    param(
        [string] $JobId,
        [string] $Reason = ""
    )

    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $global:WindmillConnection.CancelJob($JobId, $Reason)
}

function New-WindmillToken([TimeSpan] $Duration = (New-TimeSpan -Days 1)) {
    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $global:WindmillConnection.CreateToken([DateTime]::Now.Add($Duration))
}

function Get-WindmillIdToken {
    param(
        [string] $Audience
    )

    if (-not $global:WindmillConnection) {
        throw "Windmill connection not established. Run Connect-Windmill first."
    }

    return $global:WindmillConnection.GetIdToken($Audience)
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
            "Content-Type" = "application/json"
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

    [string] GetVariable([string] $Path) {
        $response = $this.Get("/w/$($this.Workspace)/variables/get_value/$Path", $true)
        return $response.Content | ConvertFrom-Json
    }

    [void] SetVariable([string] $Path, [string] $Value, [boolean] $Secret) {
        $response = $this.Get("/w/$($this.Workspace)/variables/get_value/$Path", $false)

            if ($response.StatusCode -eq 404) {
                $this.Post("/w/$($this.Workspace)/variables/create", @{ "path" = $Path; "value" = $Value; "is_secret" = $Secret; "description" = "" }, $true)
                return
            } else {
                $this.Post("/w/$($this.Workspace)/variables/update/$Path", @{ "value" = $Value }, $true)
            }
    }

    [PSCustomObject] GetResult([string] $JobId, [boolean] $AssertResultIsNotNull) {
        $response = $this.Get("/w/$($this.Workspace)/jobs_u/completed/get_result/$JobId", $true)
        $result = $response.Content | ConvertFrom-Json
        if ($AssertResultIsNotNull -and -not $result) {
            throw "result is null for job $JobId"
        }
        
        return $result
    }



    [void] SetResource([string] $Path, [Hashtable] $Value, [string] $ResourceType) {
        $response = $this.Get("/w/$($this.Workspace)/resources/get/$Path", $false)

        if ($response.StatusCode -eq 404) {
            $this.Post("/w/$($this.Workspace)/resources/create", @{ "path" = $Path; "value" = $Value; "resource_type" = $ResourceType }, $true)
            return
        } else {
            $this.Post("/w/$($this.Workspace)/resources/update_value/$Path", @{ "value" = $Value }, $true)
        }
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

    [string] CancelJob([string] $JobId, [string] $Reason) {
        return $this.Post("/w/$($this.Workspace)/jobs_u/queue/cancel/$JobId", @{ "reason" = $Reason }, $true)
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
        } elseif ($Hash) {
            $endpoint = "/w/$($this.Workspace)/jobs/run/h/$Hash"
        } else {
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
        # if ($env:WM_JOB_ID) {
        #     $params["parent_job"] = $env:WM_JOB_ID
        # }
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
            "running" = "true"
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

    [PSCustomObject] GetJob([string] $JobId) {
        $response = $this.Get("/w/$($this.Workspace)/jobs_u/get/$JobId", $true)
        return $response.Content | ConvertFrom-Json
    }

    [string] CreateToken([datetime] $Expiration) {
        $endpoint = "users/tokens/create"
        $refresh = Get-Date (Get-Date).ToUniversalTime() -UFormat %s
        $payload = @{
            "label" = "refresh $refresh"
            "expiration" = $Expiration.ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
        }

        return $this.Post($endpoint, $payload, $true).Content
    }

    [string] GetIdToken([string] $Audience) {
        return $this.Post("/w/$($this.Workspace)/oidc/token/$Audience").Content
    }
}
