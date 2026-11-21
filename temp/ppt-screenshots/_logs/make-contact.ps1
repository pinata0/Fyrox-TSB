param(
    [Parameter(Mandatory=$true)][string[]]$Patterns,
    [Parameter(Mandatory=$true)][string]$Output,
    [int]$Columns = 4
)
Add-Type -AssemblyName System.Drawing
$files = @($Patterns | ForEach-Object { Get-ChildItem -Path $_ -File } | Sort-Object FullName -Unique)
if ($files.Count -eq 0) { throw 'No screenshots found.' }
$tileW = 400
$imageH = 250
$tileH = 280
$rows = [int][Math]::Ceiling($files.Count / [double]$Columns)
$canvas = [System.Drawing.Bitmap]::new($tileW * $Columns, $tileH * $rows)
$graphics = [System.Drawing.Graphics]::FromImage($canvas)
$graphics.Clear([System.Drawing.Color]::FromArgb(25,29,36))
$graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$font = [System.Drawing.Font]::new('Malgun Gothic', 10)
try {
    for ($i=0; $i -lt $files.Count; $i++) {
        $x = ($i % $Columns) * $tileW
        $y = [int][Math]::Floor($i / $Columns) * $tileH
        $shot = [System.Drawing.Image]::FromFile($files[$i].FullName)
        try {
            $scale = [Math]::Min(($tileW-8)/[double]$shot.Width, $imageH/[double]$shot.Height)
            $drawW = [int]($shot.Width*$scale)
            $drawH = [int]($shot.Height*$scale)
            $graphics.DrawImage($shot, [int]($x+($tileW-$drawW)/2), $y, $drawW, $drawH)
        } finally { $shot.Dispose() }
        $label = $files[$i].BaseName -replace '^preview-', '' -replace '-1920x1200-', '-' -replace '-menu$', '' -replace '-game$', ''
        $graphics.DrawString($label, $font, [System.Drawing.Brushes]::White, [float]($x+6), [float]($y+$imageH+3))
    }
    $canvas.Save($Output, [System.Drawing.Imaging.ImageFormat]::Png)
} finally { $font.Dispose(); $graphics.Dispose(); $canvas.Dispose() }
Write-Output "$Output ($($files.Count) screenshots)"
