$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$projectRoot = Split-Path -Parent $PSScriptRoot
$font = [System.Drawing.Font]::new('Segoe UI', 11)
$brush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(35,45,55))
try {
    foreach ($directory in @('inputs', 'inputs-masked', 'prepared-preview')) {
        $folder = Join-Path $projectRoot "assets/character-inputs/$directory"
        $sources = @(Get-ChildItem -LiteralPath $folder -File -Filter '*.png' | Sort-Object Name)
        $canvas = [System.Drawing.Bitmap]::new(1200, 1050)
        $graphics = [System.Drawing.Graphics]::FromImage($canvas)
        try {
            $graphics.Clear([System.Drawing.Color]::FromArgb(225,229,232))
            $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            for ($i = 0; $i -lt $sources.Count; $i++) {
                $source = [System.Drawing.Image]::FromFile($sources[$i].FullName)
                try {
                    $scale = [Math]::Min(220.0 / $source.Width, 312.0 / $source.Height)
                    $width = [int]($source.Width * $scale)
                    $height = [int]($source.Height * $scale)
                    $x = ($i % 5) * 240 + [int]((240-$width)/2)
                    $y = [int][Math]::Floor($i / 5) * 350 + 8 + [int]((312-$height)/2)
                    $graphics.DrawImage($source, [System.Drawing.Rectangle]::new($x,$y,$width,$height))
                    $graphics.DrawString($sources[$i].BaseName,$font,$brush,($i % 5)*240+12,[int][Math]::Floor($i/5)*350+327)
                } finally { $source.Dispose() }
            }
            $canvas.Save((Join-Path $folder 'contact-sheet.jpg'), [System.Drawing.Imaging.ImageFormat]::Jpeg)
        } finally { $graphics.Dispose(); $canvas.Dispose() }
    }
} finally { $font.Dispose(); $brush.Dispose() }
Write-Output 'INPUT_CONTACT_SHEETS_OK: 3 sheets rebuilt from current PNGs'
