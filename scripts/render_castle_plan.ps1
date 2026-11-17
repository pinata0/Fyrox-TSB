$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$projectRoot = Split-Path -Parent $PSScriptRoot
$layout = Get-Content -LiteralPath (Join-Path $projectRoot 'assets/maps/fairy-kingdom-castle/layout.json') -Raw -Encoding utf8 | ConvertFrom-Json
$out = Join-Path $projectRoot 'artifacts/fairy-kingdom-castle/layout-plan.png'
$bmp = [System.Drawing.Bitmap]::new(1550,1180)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = 'AntiAlias'
$g.Clear([System.Drawing.ColorTranslator]::FromHtml('#F6F3E9'))
$title = [System.Drawing.Font]::new('Segoe UI',28,[System.Drawing.FontStyle]::Bold)
$head = [System.Drawing.Font]::new('Segoe UI',18,[System.Drawing.FontStyle]::Bold)
$font = [System.Drawing.Font]::new('Segoe UI',13)
$small = [System.Drawing.Font]::new('Segoe UI',11)
$ink = [System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml('#193F42'))
function Label([string]$text,[float]$x,[float]$y,$f=$font) { $g.DrawString($text,$f,$ink,$x,$y) }
function PlanX([float]$x) { return [float](65+($x+22)*10) }
function PlanY([float]$z) { return [float](1100-($z+54)*10) }
function Rect([float]$x,[float]$z,[float]$w,[float]$d,[string]$color,[int]$alpha=255) {
    $c=[System.Drawing.ColorTranslator]::FromHtml($color)
    $b=[System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb($alpha,$c))
    $g.FillRectangle($b,(PlanX ($x-$w/2)),(PlanY ($z+$d/2)),[float]($w*10),[float]($d*10)); $b.Dispose()
}
Label 'FAIRY KINGDOM CASTLE' 60 25 $title
Label 'CONCEPT SPACES  /  RETAINED ANUBIS TURN, FLANKS AND HEIGHTS  /  v3' 64 78
$polygon = [System.Drawing.PointF[]]@($layout.footprint | ForEach-Object { [System.Drawing.PointF]::new((PlanX $_[0]),(PlanY $_[1])) })
$ground=[System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml('#E1E6D6'))
$g.FillPolygon($ground,$polygon);$ground.Dispose()
foreach($b in $layout.blocks) {
    $lo=[double]$b.center[1]-[double]$b.size[1]/2
    if ($b.name -match 'roof|lintel|boundary|escarpment|balustrade' -or $lo -gt 4.4) { continue }
    if($lo -gt 3 -and $b.size[1] -le .35) { Rect $b.center[0] $b.center[2] $b.size[0] $b.size[2] '#60A6C2' 160 }
    else { Rect $b.center[0] $b.center[2] $b.size[0] $b.size[2] '#617D78' 230 }
}
$colors=@{main='#DB8732';flank='#8973B2';high='#237EAE';visit='#439C74'}
foreach($route in $layout.routes) {
    $pen=[System.Drawing.Pen]::new([System.Drawing.ColorTranslator]::FromHtml($colors[$route.kind]),3)
    $pen.DashStyle='Dash';$pen.EndCap='ArrowAnchor'
    for($j=1;$j -lt $route.points.Count;$j++) {
        $a=$route.points[$j-1];$b=$route.points[$j]
        $g.DrawLine($pen,(PlanX $a[0]),(PlanY $a[2]),(PlanX $b[0]),(PlanY $b[2]))
    };$pen.Dispose()
}
foreach($s in @(@('attack_a','ATK START'),@('attack_b','ATK FORWARD'),@('defense_b','DEF SPAWN'))) {
    $p=$layout.spawns.($s[0]); Rect $p[0] $p[2] 13 6 '#3BAE9C' 165
    Label $s[1] ((PlanX $p[0])-55) ((PlanY $p[2])-12) $small
}
for($i=0;$i -lt 2;$i++) {
    $p=$layout.points[$i];$pen=[System.Drawing.Pen]::new([System.Drawing.ColorTranslator]::FromHtml('#D49231'),4)
    $g.DrawEllipse($pen,((PlanX $p[0])-38),((PlanY $p[2])-38),76,76);$pen.Dispose()
    Label ([string][char](65+$i)) ((PlanX $p[0])-13) ((PlanY $p[2])-26) $title
}
Label '3.6 m' (PlanX -13) (PlanY 14) $small
Label '3.6 m' (PlanX 9) (PlanY 4) $small
Label 'No diagonal shortcut' 575 650 $head
Label 'Garden escarpment / outside playable footprint' 525 690 $small
Label 'READING THE LAYOUT' 1020 175 $head
$notes=@('01  Bedroom sanctuary','Bed, curtains, bedside table and rug.','','02  Town approach','West study / east cafe terrace.','Festival tree on the village green.','','03  A tree atrium','Offset doors preserve sight breaks.','Balconies and indoor dogleg.','','04  B castle interiors','North guardian hall and library.','South vaulted stone corridor.','','05  Throne and vertical routes','Open B hall / raised red throne.','A balconies and windows at 3.6 m.','','06  Spawn protection retained','Own active sanctuary: invulnerable,','excluded from AI target selection.')
$y=220;foreach($line in $notes) {Label $line 1025 $y;$y+=27}
Label 'Orange: main  /  Violet: flank' 1025 880
Label 'Blue: stairs and upper circulation' 1025 910
Label 'Green: new furnished room visits' 1025 940
Label 'Walls and routes from layout.json.' 1025 970 $small
Label 'Topology reference: Joe Iz Gaming plan;' 1025 1000 $small
Label 'Orhin_InsaLan A/B diagrams.' 1025 1025 $small
Label 'Dimensions adapted to TSB, not a scan.' 1025 1055 $small
Label '2026-11-14   |   fairy-kingdom-castle' 1025 1115 $small
$bmp.Save($out,[System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose();$bmp.Dispose();$title.Dispose();$head.Dispose();$font.Dispose();$small.Dispose();$ink.Dispose()
Write-Output $out
