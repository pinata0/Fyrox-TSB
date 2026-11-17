$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$projectRoot = Split-Path -Parent $PSScriptRoot
$canvas = [System.Drawing.Bitmap]::new(1260, 610)
$graphics = [System.Drawing.Graphics]::FromImage($canvas)
$graphics.Clear([System.Drawing.Color]::FromArgb(229, 240, 246))
$graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$font = [System.Drawing.Font]::new('Malgun Gothic', 22, [System.Drawing.FontStyle]::Bold)
$items = @(@('01', 'naia', '나이아'), @('02', 'erpin', '에르핀'), @('07', 'renua', '리뉴아'))
try {
    for ($i = 0; $i -lt $items.Count; $i++) {
        $item = $items[$i]
        $path = Join-Path $projectRoot ('artifacts/preview-selection-{0}-{1}-0-menu.png' -f $item[0], $item[1])
        $source = [System.Drawing.Image]::FromFile($path)
        try {
            $crop = [System.Drawing.Rectangle]::new(770, 0, 540, 740)
            $dest = [System.Drawing.Rectangle]::new($i*420+5, 0, 410, 562)
            $graphics.DrawImage($source, $dest, $crop, [System.Drawing.GraphicsUnit]::Pixel)
            $graphics.DrawString($item[2], $font, [System.Drawing.Brushes]::DarkSlateGray, $i*420+158, 568)
        } finally { $source.Dispose() }
    }
    $canvas.Save((Join-Path $projectRoot 'artifacts/tripo-roster-import/ingame-three.png'), [System.Drawing.Imaging.ImageFormat]::Png)
} finally { $graphics.Dispose(); $canvas.Dispose(); $font.Dispose() }
