$global:WindmillConnection = $null

function Connect-Windmill {
    param(
        [string] $BaseUrl = $null,
        [string] $Token = $null, 
        [string] $Workspace = $null
    )

    $global:WindmillConnection = [Windmill]::new($BaseUrl, $Token, $Workspace)
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

class Windmill {
    [string] $BaseUrl
    [string] $Token
    [string] $Workspace
    [Hashtable] $Headers

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
    }

    [Object] Get([string] $Endpoint, [boolean] $RaiseForStatus = $true) {
        $Url = "$($this.BaseUrl)/$($Endpoint.TrimStart('/'))"
        $Response = Invoke-WebRequest -Uri $Url -Method "GET" -Headers $this.Headers -SkipHttpErrorCheck
        
        if ($RaiseForStatus -and -not $Response.BaseResponse.IsSuccessStatusCode) {
            throw "Request failed with status code $($Response.StatusCode)"
        }

        return $Response
    }

    [Object] Post([string] $Endpoint, [Object] $Body = $null, [boolean] $RaiseForStatus = $true) {
        $Url = "$($this.BaseUrl)/$($Endpoint.TrimStart('/'))"
        $Response = Invoke-WebRequest -Uri $Url -Method "POST" -Headers $this.Headers -Body ($Body | ConvertTo-Json) -SkipHttpErrorCheck -ContentType "application/json"
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

    [PSCustomObject] GetResult([string] $JobId, [boolean] $AssertResultIsNotNull = $true) {
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
}
