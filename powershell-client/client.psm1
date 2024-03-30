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
        [Parameter(Mandatory = $true)]
        [string] $Path
    )

    if ($null -eq $global:WindmillConnection) {
        throw "Please connect to Windmill using Connect-Windmill first."
    }

    return $global:WindmillConnection.GetVariable($Path)
}

function Get-WindmillResource {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Path
    )

    if ($null -eq $global:WindmillConnection) {
        throw "Please connect to Windmill using Connect-Windmill first."
    }

    return $global:WindmillConnection.GetResource($Path)
}

function Get-WindmillVersion {
  if ($null -eq $global:WindmillConnection) {
      throw "Please connect to Windmill using Connect-Windmill first."
  }

  return $global:WindmillConnection.Version
}

function Get-WindmillUser {
  if ($null -eq $global:WindmillConnection) {
      throw "Please connect to Windmill using Connect-Windmill first."
  }

  return $global:WindmillConnection.GetWindmillUser()
}

function Get-WindmillWorkspace {
  if ($null -eq $global:WindmillConnection) {
      throw "Please connect to Windmill using Connect-Windmill first."
  }

  return $global:WindmillConnection.Workspace
}

function Get-WindmillRootJobId {
  param(
    [Parameter(Mandatory=$false)]
    [String] $JobId
  )

  return $global:WindmillConnection.GetRootJobId($JobId)
}

function Get-WindmillResult {
  param(
    [Parameter(Mandatory=$true)]
    [String] $JobId,
    [switch] $AssertResultIsNotNull
  )

  return $global:WindmillConnection.GetResult($JobId, $AssertResultIsNotNull)
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

    [object] SendRequest(
        [string] $Method,
        [string] $Endpoint
    ) {
        $url = "$($this.BaseUrl)/$($Endpoint.TrimStart('/'))"
        try {
            return Invoke-RestMethod -Method $Method -Uri $url -Headers $this.Headers
        } catch {
            $StatusCode = $_.Exception.Response.StatusCode.value__ 
            $Text = $_.ErrorDetails.Message
            $ErrMsg = "$url : $StatusCode, $Text"
            Write-Error $ErrMsg
            throw $_.Exception
        }
    }

    [string] GetVariable([string] $Path) {
        return $this.SendRequest("GET", "/w/$($this.Workspace)/variables/get_value/$Path")
    }

    [PSCustomObject] GetResource([string] $Path) {
        return $this.SendRequest("GET", "/w/$($this.Workspace)/resources/get_value_interpolated/$Path")
    }

    [string] Version() {
      return $this.SendRequest("GET", "/version").text
    }

    [PSCustomObject]GetWindmillUser() {
      return $this.SendRequest("GET", "users/whoami")
    }

    [string] GetRootJobId([string] $JobId) {
      if (-not $JobId) {
        $JobId = $env:WM_JOB_ID
      }

        return $this.SendRequest("GET", "/w/$($this.Workspace)/jobs_u/get_root_job_id/$JobId")
    }

    [PSCustomObject] GetResult([string] $JobId, [boolean] $AssertResultIsNotNull) {
      $result = $this.SendRequest("GET", "w/$($this.Workspace)/jobs_u/completed/get_result/$JobId")
      if ($AssertResultIsNotNull -and -not $result) {
        throw "result is null for $JobId"
      }

      return $result
    }
}
