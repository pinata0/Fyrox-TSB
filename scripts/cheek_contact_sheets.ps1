param([string]$Output = 'artifacts/cheek-interaction')
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$roster = Get-Content (Join-Path $root 'assets/references/manifest.json') -Raw | ConvertFrom-Json
$font = [System.Drawing.Font]::new('Malgun Gothic',16)
$brush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
try {
    for ($page=0; $page -lt 3; $page++) {
        $canvas=[System.Drawing.Bitmap]::new(1200,1600)
        $g=[System.Drawing.Graphics]::FromImage($canvas)
        $g.Clear([System.Drawing.Color]::FromArgb(30,42,58))
        $g.InterpolationMode=[System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        try {
            for ($row=0; $row -lt 5; $row++) {
                $id=$page*5+$row
                for ($col=0; $col -lt 3; $col++) {
                    $phase=@(0,1,4)[$col]
                    $file='artifacts/preview-cheeks-{0:00}-{1}-{2}-menu.png' -f $id,$roster[$id].slug,$phase
                    $source=[System.Drawing.Image]::FromFile((Join-Path $root $file))
                    try {
                        $crop=[System.Drawing.Rectangle]::new([int]($source.Width*.27),[int]($source.Height*.14),[int]($source.Width*.41),[int]($source.Height*.48))
                        $dest=[System.Drawing.Rectangle]::new($col*400,$row*320+35,400,280)
                        $g.DrawImage($source,$dest,$crop,[System.Drawing.GraphicsUnit]::Pixel)
                    } finally {$source.Dispose()}
                    $g.DrawString(($roster[$id].name+' / '+@('기본','당기기','놓은 뒤')[$col]),$font,$brush,$col*400+8,$row*320+5)
                }
            }
            $canvas.Save((Join-Path $root "$Output/contact-$page.png"),[System.Drawing.Imaging.ImageFormat]::Png)
        } finally {$g.Dispose();$canvas.Dispose()}
    }
} finally {$font.Dispose();$brush.Dispose()}
