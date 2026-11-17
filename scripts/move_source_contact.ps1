$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$records = Get-Content (Join-Path $root 'artifacts/move-motions/research/source-analysis.json') -Raw | ConvertFrom-Json
$canvas = [System.Drawing.Bitmap]::new(1350, 2100)
$g = [System.Drawing.Graphics]::FromImage($canvas)
$g.Clear([System.Drawing.Color]::WhiteSmoke)
$g.SmoothingMode = 'AntiAlias'
$font = [System.Drawing.Font]::new('Segoe UI', 12)
$chains = @(
    @{ names = @('Hips','Torso','Head'); color = 'DarkSlateGray' },
    @{ names = @('Torso','UpperArm.L','Forearm.L','Hand.L'); color = 'IndianRed' },
    @{ names = @('Torso','UpperArm.R','Forearm.R','Hand.R'); color = 'DarkOrange' },
    @{ names = @('Hips','Thigh.L','Shin.L','Foot.L'); color = 'RoyalBlue' },
    @{ names = @('Hips','Thigh.R','Shin.R','Foot.R'); color = 'SeaGreen' }
)
for ($row = 0; $row -lt 15; $row++) {
    $r = $records[$row]
    $poses = @(0,.2,.4,.6,.8 | ForEach-Object { $r.points[[Math]::Round(($r.points.Count-1)*$_)] })
    $xs = @($poses | ForEach-Object { $_.PSObject.Properties.Value | ForEach-Object { $_[0] } })
    $ys = @($poses | ForEach-Object { $_.PSObject.Properties.Value | ForEach-Object { $_[1] } })
    $xmin = ($xs | Measure-Object -Minimum).Minimum; $xmax = ($xs | Measure-Object -Maximum).Maximum
    $ymin = ($ys | Measure-Object -Minimum).Minimum; $ymax = ($ys | Measure-Object -Maximum).Maximum
    $scale = [Math]::Min(150/[Math]::Max(1,$xmax-$xmin),110/[Math]::Max(1,$ymax-$ymin))
    $g.DrawString(('{0}  {1:N3}s' -f $r.slug,$r.duration),$font,[System.Drawing.Brushes]::Black,8,($row*140+6))
    for ($col = 0; $col -lt 5; $col++) {
        $p = $poses[$col]
        foreach ($chain in $chains) {
            $pen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromName($chain.color),3)
            $points = @($chain.names | ForEach-Object {
                $v = $p.$_
                [System.Drawing.PointF]::new((220+$col*225+($v[0]-($xmin+$xmax)/2)*$scale),($row*140+128-($v[1]-$ymin)*$scale))
            })
            $g.DrawLines($pen,[System.Drawing.PointF[]]$points)
            $pen.Dispose()
        }
    }
}
$canvas.Save((Join-Path $root 'artifacts/move-motions/research/source-poses.png'))
$g.Dispose(); $canvas.Dispose(); $font.Dispose()
