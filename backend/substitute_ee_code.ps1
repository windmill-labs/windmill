param(
    [switch]$Revert,
    [switch]$Copy,
    [switch]$MoveNewFiles,
    [string]$Dir
)

$ErrorActionPreference = "Stop"
$scriptDirPath = (Get-Location).Path
$rootDirPath = (Join-Path $scriptDirPath "..")

$REVERT = $true
$COPY = $false
$MOVE_NEW_FILES = $true
$EE_CODE_DIR = "../../windmill-ee-private/"



if ($Revert) {
    $REVERT = $true
    $MOVE_NEW_FILES = $true
}

if ($Copy) {
    $COPY = $true
}

if ($MoveNewFiles) {
    $MOVE_NEW_FILES = $true
}

if ($Dir) {
    $EE_CODE_DIR = $Dir
}

if (-not (Test-Path $EE_CODE_DIR)) {
    if (-not (Test-Path (Join-Path $rootDirPath $EE_CODE_DIR))) {
        $EE_CODE_DIR = Join-Path $rootDirPath $EE_CODE_DIR
    }
}

Write-Host "EE code directory = $EE_CODE_DIR | Revert = $REVERT"

if (-not (Test-Path $EE_CODE_DIR)) {
    Write-Host "Windmill EE repo not found, please clone it next to this repository (or use the --dir option) and try again"
    Write-Host ">   git clone git@github.com:windmill-labs/windmill-ee-private.git"
    exit
}

if ($REVERT) {
    Get-ChildItem -Path $EE_CODE_DIR -Recurse -Filter "*ee.rs" | ForEach-Object {
        $eeFile = $_.FullName
        $ceFile = $eeFile.Replace("windmill-ee-private", "windmill/backend")
        Remove-Item -Path $ceFile -Force -ErrorAction SilentlyContinue
    }
} elseif (-not $MOVE_NEW_FILES) {
    Get-ChildItem -Path $EE_CODE_DIR -Recurse -Filter "*ee.rs" | ForEach-Object {
        $eeFile = $_.FullName
        $ceFile = $eeFile.Replace("windmill-ee-private", "windmill/backend")
        if ($COPY) {
            Copy-Item -Path $eeFile -Destination $ceFile
            Write-Host "File copied '$eeFile' -->> '$ceFile'"
        } else {
            # Ensure that the paths are correct and fully qualified before creating the symlink
            
            $eeFileFullPath = [System.IO.Path]::GetFullPath($eeFile)
            $ceFileFullPath = [System.IO.Path]::GetFullPath($ceFile)
            
            # Now create the symbolic link
            New-Item -ItemType SymbolicLink -Path $ceFileFullPath -Target $eeFileFullPath -Force
            Write-Host "Symlink created '$eeFileFullPath' -->> '$ceFileFullPath'"
        }
    }
}

if ($MOVE_NEW_FILES) {
    Get-ChildItem -Path (Join-Path $rootDirPath "backend/") -Recurse -Filter "*ee.rs" | ForEach-Object {
        $ceFile = $_.FullName
        $eeFile = $ceFile.Replace("windmill/backend", "windmill-ee-private")
        if (-not (Test-Path $eeFile)) {
            Move-Item -Path $ceFile -Destination $eeFile
            if (-not $REVERT) {
                $eeFileFullPath = [System.IO.Path]::GetFullPath($eeFile)
                $ceFileFullPath = [System.IO.Path]::GetFullPath($ceFile)

                # Create the symlink correctly
                New-Item -ItemType SymbolicLink -Path $ceFileFullPath -Target $eeFileFullPath -Force
            }
            Write-Host "File moved '$ceFile' -->> '$eeFile'"
        }
    }
}

