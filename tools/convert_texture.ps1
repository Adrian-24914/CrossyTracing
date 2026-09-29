param(
    [Parameter(Mandatory = $true)]
    [string]$Source,

    [Parameter(Mandatory = $true)]
    [string]$Destination,

    [int]$Size = 256
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
Add-Type -AssemblyName System.Drawing

$sourcePath = [System.IO.Path]::GetFullPath($Source)
$destinationPath = [System.IO.Path]::GetFullPath($Destination)
$temporaryPath = "$destinationPath.tmp.bmp"
$sourceImage = [System.Drawing.Image]::FromFile($sourcePath)

try {
    $runtimeImage = New-Object System.Drawing.Bitmap(
        $Size,
        $Size,
        [System.Drawing.Imaging.PixelFormat]::Format24bppRgb
    )
    try {
        $graphics = [System.Drawing.Graphics]::FromImage($runtimeImage)
        try {
            $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
            $graphics.DrawImage($sourceImage, 0, 0, $Size, $Size)
        }
        finally {
            $graphics.Dispose()
        }

        $runtimeImage.Save($temporaryPath, [System.Drawing.Imaging.ImageFormat]::Bmp)
    }
    finally {
        $runtimeImage.Dispose()
    }
}
finally {
    $sourceImage.Dispose()
}

Move-Item -LiteralPath $temporaryPath -Destination $destinationPath -Force
