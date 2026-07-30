# Build MSI installer for Nexus Remote Client
# This script collects the binary + all GStreamer DLLs and creates an MSI using WiX

param(
    [string]$BinaryPath = "target\release\wot-remote-client.exe",
    [string]$OutputMsi = "nexus-remote-client-windows-x86_64.msi",
    [string]$Version = "0.1.0"
)

$ErrorActionPreference = "Stop"

# ── 1. Find GStreamer installation ──
$gstRoot = $null
$searchPaths = @(
    "C:\gstreamer\1.0\msvc_x86_64",
    "$env:GSTREAMER_1_0_ROOT_MSVC_X86_64",
    "$env:ProgramFiles\GStreamer\1.0\msvc_x86_64"
)

foreach ($p in $searchPaths) {
    if ($p -and (Test-Path "$p\bin")) {
        $gstRoot = $p
        break
    }
}

if (-not $gstRoot) {
    Write-Error "GStreamer installation not found!"
    exit 1
}

Write-Host "Found GStreamer at: $gstRoot"

# ── 2. Create staging directory ──
$stage = "msi-staging"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path "$stage\bin" -Force | Out-Null
New-Item -ItemType Directory -Path "$stage\lib\gstreamer-1.0" -Force | Out-Null

# Copy our binary
Copy-Item $BinaryPath "$stage\bin\nexus-remote-client.exe"
Write-Host "Copied binary"

# Copy GStreamer core DLLs
Copy-Item "$gstRoot\bin\*.dll" "$stage\bin\" -Force
Write-Host "Copied $(( Get-ChildItem "$stage\bin\*.dll" ).Count) GStreamer DLLs"

# Copy GStreamer plugins (only the ones we need for WebRTC video streaming)
$essentialPlugins = @(
    "gstcoreelements", "gstcoretracers", "gstapp", "gstplayback",
    "gstwebrtc", "gstdtls", "gstsrtp", "gstsctp",
    "gstnice", "gstrtp", "gstrtpmanager",
    "gstvideotestsrc", "gstvideoconvertscale", "gstvideorate", "gstvideoparsersbad",
    "gstx264", "gstopenh264", "gstvideocodectestsink",
    "gstautodetect", "gstd3d11",
    "gstaudioconvert", "gstaudioresample", "gstopus", "gstaudiotestsrc",
    "gstvolume", "gstlevel",
    "gsttypefindfunctions", "gstaudioparsers", "gstvideoparsers",
    "gstgio", "gstisomp4"
)

$pluginDir = "$gstRoot\lib\gstreamer-1.0"
$copiedPlugins = 0
foreach ($plugin in $essentialPlugins) {
    $dll = Get-ChildItem "$pluginDir\$plugin.dll" -ErrorAction SilentlyContinue
    if ($dll) {
        Copy-Item $dll.FullName "$stage\lib\gstreamer-1.0\" -Force
        $copiedPlugins++
    }
}
# Also copy any remaining plugins that might be needed (safe fallback)
Copy-Item "$pluginDir\*.dll" "$stage\lib\gstreamer-1.0\" -Force -ErrorAction SilentlyContinue
$totalPlugins = (Get-ChildItem "$stage\lib\gstreamer-1.0\*.dll").Count
Write-Host "Copied $totalPlugins GStreamer plugins"

# ── 3. Generate WiX source XML ──

# Generate unique but deterministic GUIDs from file names
function Get-DeterministicGuid($name) {
    $md5 = [System.Security.Cryptography.MD5]::Create()
    $bytes = [System.Text.Encoding]::UTF8.GetBytes("nexus-remote-$name")
    $hash = $md5.ComputeHash($bytes)
    return [guid]::new($hash).ToString().ToUpper()
}

$productGuid = Get-DeterministicGuid "product-$Version"
$upgradeGuid = "E8B7C5A1-3D2F-4A6B-9C8E-1F0D2E3B4A5C"  # Fixed — never change this

# Build file entries for DLLs in bin/
$binDlls = Get-ChildItem "$stage\bin\*.dll"
$binComponents = ""
$binComponentRefs = ""
foreach ($dll in $binDlls) {
    $id = "bin_" + ($dll.Name -replace '[^a-zA-Z0-9]', '_')
    $guid = Get-DeterministicGuid $dll.Name
    $binComponents += @"

              <Component Id="$id" Guid="$guid">
                <File Source="msi-staging\bin\$($dll.Name)" />
              </Component>
"@
    $binComponentRefs += "      <ComponentRef Id=`"$id`" />`n"
}

# Build file entries for plugins in lib/gstreamer-1.0/
$plugins = Get-ChildItem "$stage\lib\gstreamer-1.0\*.dll"
$pluginComponents = ""
$pluginComponentRefs = ""
foreach ($plugin in $plugins) {
    $id = "plugin_" + ($plugin.Name -replace '[^a-zA-Z0-9]', '_')
    $guid = Get-DeterministicGuid "plugin-$($plugin.Name)"
    $pluginComponents += @"

              <Component Id="$id" Guid="$guid">
                <File Source="msi-staging\lib\gstreamer-1.0\$($plugin.Name)" />
              </Component>
"@
    $pluginComponentRefs += "      <ComponentRef Id=`"$id`" />`n"
}

$wxs = @"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">
  <Package Name="Nexus Remote Client"
           Manufacturer="Nexus Remote"
           Version="$Version"
           UpgradeCode="$upgradeGuid"
           Compressed="yes">

    <MajorUpgrade DowngradeErrorMessage="A newer version is already installed." />
    <MediaTemplate EmbedCab="yes" />

    <!-- Install directory structure -->
    <StandardDirectory Id="ProgramFiles64Folder">
      <Directory Id="INSTALLFOLDER" Name="Nexus Remote">
        <Directory Id="BinFolder" Name="bin">
          <!-- Main executable -->
          <Component Id="MainExe" Guid="$(Get-DeterministicGuid 'main-exe')">
            <File Id="NexusRemoteExe" Source="msi-staging\bin\nexus-remote-client.exe" KeyPath="yes" />
          </Component>

          <!-- GStreamer DLLs -->
          $binComponents
        </Directory>

        <Directory Id="LibFolder" Name="lib">
          <Directory Id="GstPluginFolder" Name="gstreamer-1.0">
            $pluginComponents
          </Directory>
        </Directory>
      </Directory>
    </StandardDirectory>

    <!-- Start Menu shortcut -->
    <StandardDirectory Id="ProgramMenuFolder">
      <Component Id="StartMenuShortcut" Guid="$(Get-DeterministicGuid 'shortcut')">
        <Shortcut Id="NexusShortcut"
                  Name="Nexus Remote Client"
                  Target="[BinFolder]nexus-remote-client.exe"
                  WorkingDirectory="BinFolder" />
        <RegistryValue Root="HKCU" Key="Software\NexusRemote" Name="installed" Type="integer" Value="1" KeyPath="yes" />
        <RemoveFolder Id="RemoveMenuFolder" On="uninstall" />
      </Component>
    </StandardDirectory>

    <!-- Add bin to PATH -->
    <Component Id="PathEntry" Directory="INSTALLFOLDER" Guid="$(Get-DeterministicGuid 'path')">
      <Environment Id="PATH" Name="PATH" Value="[BinFolder]" Permanent="no" Part="last" Action="set" System="yes" />
      <Environment Id="GST_PLUGIN_PATH" Name="GST_PLUGIN_PATH" Value="[GstPluginFolder]" Permanent="no" Part="last" Action="set" System="yes" />
      <RegistryValue Root="HKLM" Key="Software\NexusRemote" Name="envset" Type="integer" Value="1" KeyPath="yes" />
    </Component>

    <!-- Feature definition -->
    <Feature Id="Complete" Title="Nexus Remote Client" Level="1">
      <ComponentRef Id="MainExe" />
      <ComponentRef Id="StartMenuShortcut" />
      <ComponentRef Id="PathEntry" />
$binComponentRefs$pluginComponentRefs
    </Feature>
  </Package>
</Wix>
"@

$wxsPath = "nexus-remote.wxs"
$wxs | Out-File -FilePath $wxsPath -Encoding UTF8
Write-Host "Generated WiX source with $(($binDlls.Count + $plugins.Count)) files"

# ── 4. Build MSI ──
Write-Host "Installing WiX toolset..."
dotnet tool install --global wix

Write-Host "Building MSI..."
wix build -o $OutputMsi $wxsPath -arch x64

if (Test-Path $OutputMsi) {
    $size = [math]::Round((Get-Item $OutputMsi).Length / 1MB, 1)
    Write-Host "SUCCESS: Created $OutputMsi ($size MB)"
} else {
    Write-Error "Failed to create MSI"
    exit 1
}
