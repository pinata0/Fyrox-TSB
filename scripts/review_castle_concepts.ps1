param([string]$OutputName = 'game-contact.png')
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$names = @('bedroom','study','street','cafe','square','atrium','corridor','guardian-hall','throne')
$bmp = [System.Drawing.Bitmap]::new(1920,1320)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.Clear([System.Drawing.Color]::FromArgb(21,36,38))
$g.InterpolationMode = 'HighQualityBicubic'
$font = [System.Drawing.Font]::new('Segoe UI',15,[System.Drawing.FontStyle]::Bold)
for($i=0;$i -lt $names.Count;$i++) {
    $x=($i%3)*640; $y=[math]::Floor($i/3)*440
    $g.DrawString($names[$i].ToUpper(),$font,[System.Drawing.Brushes]::White,$x+12,$y+7)
    $source=Join-Path $root ('artifacts/preview-map-'+$names[$i]+'-game.png')
    $img=[System.Drawing.Image]::FromFile($source)
    $g.DrawImage($img,[System.Drawing.Rectangle]::new($x,$y+40,640,400))
    $img.Dispose()
}
$out=Join-Path $root ('artifacts/castle-concept-v3/'+$OutputName)
$bmp.Save($out,[System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose(); $font.Dispose()
Write-Output $out
