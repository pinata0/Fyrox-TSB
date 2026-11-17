$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$projectRoot = Split-Path -Parent $PSScriptRoot
$models = Get-Content -LiteralPath (Join-Path $projectRoot 'assets/characters/manifest.json') -Raw | ConvertFrom-Json
$references = Get-Content -LiteralPath (Join-Path $projectRoot 'assets/references/manifest.json') -Raw | ConvertFrom-Json
$font = [System.Drawing.Font]::new('Malgun Gothic', 15)
$heading = [System.Drawing.Font]::new('Segoe UI', 21, [System.Drawing.FontStyle]::Bold)
$brush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(36,52,73))
try {
    foreach ($kind in @('model', 'ingame')) {
        $canvas = [System.Drawing.Bitmap]::new(1500, 1080)
        $graphics = [System.Drawing.Graphics]::FromImage($canvas)
        $graphics.Clear([System.Drawing.Color]::FromArgb(241,238,232))
        $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.DrawString("Trickcal : Shooting Balddagu / 15 characters / $kind", $heading, $brush, 24, 14)
        try {
            for ($i = 0; $i -lt $models.Count; $i++) {
                $slug = $models[$i].slug
                $left = ($i % 5) * 300
                $top = [Math]::Floor($i / 5) * 335 + 65
                $path = if ($kind -eq 'model') { "assets/characters/$slug-preview.png" } else { 'artifacts/preview-gallery-{0:00}-{1}-dance-front.png' -f $i,$slug }
                $source = [System.Drawing.Image]::FromFile((Join-Path $projectRoot $path))
                try {
                    if ($kind -eq 'model') {
                        $graphics.DrawImage($source, [System.Drawing.Rectangle]::new($left+10,$top,280,280))
                    } else {
                        $crop = [System.Drawing.Rectangle]::new(620,230,700,740)
                        $dest = [System.Drawing.Rectangle]::new($left+18,$top,264,280)
                        $graphics.DrawImage($source,$dest,$crop,[System.Drawing.GraphicsUnit]::Pixel)
                    }
                } finally { $source.Dispose() }
                $graphics.DrawString(('{0:00}  {1}' -f $i,$references[$i].name), $font, $brush, $left+20, $top+287)
            }
            $canvas.Save((Join-Path $projectRoot "artifacts/character-$kind-contact.png"),[System.Drawing.Imaging.ImageFormat]::Png)
        } finally { $graphics.Dispose(); $canvas.Dispose() }
    }
} finally { $font.Dispose(); $heading.Dispose(); $brush.Dispose() }
Write-Output 'CONTACT_SHEETS_OK: Blender and Fyrox previews for all 15 characters'
