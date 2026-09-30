param(
    [Parameter(Mandatory = $true)] [string]$Source,
    [Parameter(Mandatory = $true)] [string]$Destination,
    [int]$MaximumWidth = 480,
    [int]$MaximumHeight = 300
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$sourceImage = [System.Drawing.Image]::FromFile([System.IO.Path]::GetFullPath($Source))
$temporaryPath = "$( [System.IO.Path]::GetFullPath($Destination) ).tmp.bmp"

try {
    $scale = [Math]::Min($MaximumWidth / $sourceImage.Width, $MaximumHeight / $sourceImage.Height)
    $width = [Math]::Max(1, [Math]::Round($sourceImage.Width * $scale))
    $height = [Math]::Max(1, [Math]::Round($sourceImage.Height * $scale))
    # El BMP de 32 bits conserva los bordes semitransparentes de los PNG.
    # Antes se usaba magenta como chroma key, lo que destruia ese degradado.
    $runtimeImage = New-Object System.Drawing.Bitmap($width, $height, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    try {
        $graphics = [System.Drawing.Graphics]::FromImage($runtimeImage)
        try {
            $graphics.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
            $graphics.Clear([System.Drawing.Color]::Transparent)
            $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $graphics.DrawImage($sourceImage, 0, 0, $width, $height)
        }
        finally { $graphics.Dispose() }
        $runtimeImage.Save($temporaryPath, [System.Drawing.Imaging.ImageFormat]::Bmp)
    }
    finally { $runtimeImage.Dispose() }
}
finally { $sourceImage.Dispose() }

Move-Item -LiteralPath $temporaryPath -Destination ([System.IO.Path]::GetFullPath($Destination)) -Force
