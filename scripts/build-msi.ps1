# Build MSI installer and Portable Package for Nexus Remote Client
# Collects binary + GStreamer DLLs and creates an MSI using WiX v4 + a portable ZIP

param(
    [string]$BinaryPath = "target\release\nexus-remote-client.exe",
    [string]$OutputMsi = "nexus-remote-client-windows-x86_64.msi",
    [string]$OutputZip = "nexus-remote-client-windows-portable-x86_64.zip",
    [string]$Version = "0.2.0"
)

$ErrorActionPreference = "Stop"

# ── 1. Find GStreamer installation ──
$gstRoot = $null
foreach ($p in @(
    "D:\gstreamer\1.0\msvc_x86_64",
    "C:\gstreamer\1.0\msvc_x86_64",
    "$env:GSTREAMER_1_0_ROOT_MSVC_X86_64",
    "$env:ProgramFiles\GStreamer\1.0\msvc_x86_64"
)) {
    if ($p -and (Test-Path "$p\bin")) {
        $gstRoot = $p
        break
    }
}
if (-not $gstRoot) {
    Write-Error "GStreamer installation directory not found! Please check GSTREAMER_1_0_ROOT_MSVC_X86_64."
    exit 1
}
Write-Host "Found GStreamer at: $gstRoot"

# ── 2. Create staging directory ──
$stage = "msi-staging"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path "$stage" -Force | Out-Null
New-Item -ItemType Directory -Path "$stage\plugins" -Force | Out-Null

if (-not (Test-Path $BinaryPath)) {
    Write-Error "Binary not found at: $BinaryPath"
    exit 1
}

# Copy binary
Copy-Item $BinaryPath "$stage\nexus-remote-client.exe" -Force

# Copy ALL GStreamer DLLs into same directory as the exe (required for DLL resolution)
Copy-Item "$gstRoot\bin\*.dll" "$stage\" -Force
$dllCount = (Get-ChildItem "$stage\*.dll").Count
Write-Host "Copied $dllCount runtime DLLs into staging"

# Copy GStreamer plugins
if (Test-Path "$gstRoot\lib\gstreamer-1.0") {
    Copy-Item "$gstRoot\lib\gstreamer-1.0\*.dll" "$stage\plugins\" -Force
    $pluginCount = (Get-ChildItem "$stage\plugins\*.dll").Count
    Write-Host "Copied $pluginCount GStreamer plugins into staging\plugins"
} else {
    Write-Warning "No plugins directory found at $gstRoot\lib\gstreamer-1.0"
}

# ── 3. Create Portable ZIP Bundle ──
Write-Host "Creating portable ZIP archive..."
Compress-Archive -Path "$stage\*" -DestinationPath $OutputZip -Force
Write-Host "Created portable ZIP: $OutputZip"

# ── 4. Install WiX v4 tool ──
Write-Host "Installing WiX toolset..."
dotnet tool install --global wix --version 4.0.5 2>$null || true
wix extension add WixToolset.UI.wixext/4.0.5 -g 2>$null || true

# ── 5. Generate WiX source ──
function SafeId($prefix, $name) {
    $safe = ($name -replace '[^a-zA-Z0-9]', '_')
    return "${prefix}_${safe}"
}

# Build component XML for all DLLs in the main directory
$mainFiles = Get-ChildItem "$stage\*.dll"
$mainComponents = ""
foreach ($f in $mainFiles) {
    $id = SafeId "D" $f.Name
    $mainComponents += "        <Component Id=`"$id`" Guid=`"*`"><File Source=`"$stage\$($f.Name)`" /></Component>`n"
}

# Build component XML for all plugins
$pluginFiles = Get-ChildItem "$stage\plugins\*.dll"
$pluginComponents = ""
foreach ($f in $pluginFiles) {
    $id = SafeId "P" $f.Name
    $pluginComponents += "        <Component Id=`"$id`" Guid=`"*`"><File Source=`"$stage\plugins\$($f.Name)`" /></Component>`n"
}

$wxs = @"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs"
     xmlns:ui="http://wixtoolset.org/schemas/v4/wxs/ui">

  <Package Name="Nexus Remote Client"
           Manufacturer="Nexus Remote"
           Version="$Version"
           UpgradeCode="E8B7C5A1-3D2F-4A6B-9C8E-1F0D2E3B4A5C"
           Scope="perMachine"
           Compressed="yes">

    <MajorUpgrade DowngradeErrorMessage="A newer version of Nexus Remote Client is already installed." />
    <MediaTemplate EmbedCab="yes" />

    <ui:WixUI Id="WixUI_InstallDir" InstallDirectory="INSTALLFOLDER" />
    <WixVariable Id="WixUILicenseRtf" Value="license.rtf" />

    <StandardDirectory Id="ProgramFiles64Folder">
      <Directory Id="INSTALLFOLDER" Name="Nexus Remote">
        <Directory Id="PluginsFolder" Name="plugins" />
      </Directory>
    </StandardDirectory>

    <ComponentGroup Id="MainComponents" Directory="INSTALLFOLDER">
      <Component Id="MainExe" Guid="*">
        <File Id="NexusRemoteExe" Source="$stage\nexus-remote-client.exe" KeyPath="yes" />
      </Component>
      <Component Id="EnvPath" Guid="A1B2C3D4-E5F6-7890-ABCD-EF1234567890">
        <Environment Id="PATH" Name="PATH" Value="[INSTALLFOLDER]" Permanent="no" Part="last" Action="set" System="yes" />
        <Environment Id="GST_PLUGIN_PATH" Name="GST_PLUGIN_PATH" Value="[PluginsFolder]" Permanent="no" Part="last" Action="set" System="yes" />
        <RegistryValue Root="HKLM" Key="Software\NexusRemote" Name="envpath" Type="integer" Value="1" KeyPath="yes" />
      </Component>
$mainComponents
    </ComponentGroup>

    <ComponentGroup Id="PluginComponents" Directory="PluginsFolder">
$pluginComponents
    </ComponentGroup>

    <Feature Id="Complete" Title="Nexus Remote Client" Level="1">
      <ComponentGroupRef Id="MainComponents" />
      <ComponentGroupRef Id="PluginComponents" />
    </Feature>

  </Package>
</Wix>
"@

$wxs | Out-File -FilePath "nexus-remote.wxs" -Encoding UTF8
Write-Host "Generated WiX source ($($mainFiles.Count + $pluginFiles.Count) bundled dependencies)"

# Minimal RTF license
$license = "{\rtf1\ansi Nexus Remote Client\par Open-source ultra-low latency remote desktop software.\par }"
$license | Out-File -FilePath "license.rtf" -Encoding ASCII

# ── 6. Build MSI ──
Write-Host "Compiling MSI installer..."
wix build -o $OutputMsi nexus-remote.wxs -ext WixToolset.UI.wixext -arch x64

if (Test-Path $OutputMsi) {
    $size = [math]::Round((Get-Item $OutputMsi).Length / 1MB, 1)
    Write-Host "SUCCESS: Built MSI installer $OutputMsi ($size MB)" -ForegroundColor Green
} else {
    Write-Error "Failed to create MSI"
    exit 1
}
