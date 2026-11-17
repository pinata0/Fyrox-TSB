param([int]$First = 0, [int]$Count = 5)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$records = Get-Content (Join-Path $root 'assets/characters/move/motions.json') -Raw | ConvertFrom-Json
$canvas = [System.Drawing.Bitmap]::new(1400, (340*$Count))
$g = [System.Drawing.Graphics]::FromImage($canvas)
$g.Clear([System.Drawing.Color]::WhiteSmoke)
$g.InterpolationMode = 'HighQualityBicubic'
$font = [System.Drawing.Font]::new('Segoe UI', 12)
$modes = @('walk','run','walk-attack','run-attack')
try {
    for ($row=0; $row -lt $Count; $row++) {
        $id = $First+$row
        for ($col=0; $col -lt 4; $col++) {
            $slug = $records[$id].slug
            $mode = $modes[$col]
            $path = Join-Path $root ('artifacts/preview-move-{0:00}-{1}-{2}-phase1-dance-front.jpg' -f $id,$slug,$mode)
            $source = [System.Drawing.Image]::FromFile($path)
            try {
                $crop = [System.Drawing.Rectangle]::new(620,280,730,650)
                $dest = [System.Drawing.Rectangle]::new(($col*350),($row*340+25),350,312)
                $g.DrawImage($source,$dest,$crop,[System.Drawing.GraphicsUnit]::Pixel)
            } finally { $source.Dispose() }
            $g.DrawString("$slug / $mode",$font,[System.Drawing.Brushes]::Black,($col*350+6),($row*340+2))
        }
    }
    $canvas.Save((Join-Path $root "artifacts/move-motions/contact-$First.png"))
} finally { $g.Dispose(); $canvas.Dispose(); $font.Dispose() }
