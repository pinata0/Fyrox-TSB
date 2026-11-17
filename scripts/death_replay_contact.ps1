Add-Type -AssemblyName System.Drawing
$projectRoot=Split-Path -Parent $PSScriptRoot
$records=Get-Content (Join-Path $projectRoot 'assets/characters/death/motions.json') -Raw | ConvertFrom-Json
$font=[System.Drawing.Font]::new('Arial',13)
try {
 for($sheet=0;$sheet -lt 3;$sheet++) {
  $canvas=[System.Drawing.Bitmap]::new(1200,1550);$graphics=[System.Drawing.Graphics]::FromImage($canvas)
  try {
   $graphics.Clear([System.Drawing.Color]::FromArgb(21,35,51))
   $graphics.InterpolationMode=[System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
   for($row=0;$row -lt 5;$row++) {
    $hero=$sheet*5+$row
    for($col=0;$col -lt 4;$col++) {
     $source=[System.Drawing.Image]::FromFile((Join-Path $projectRoot ('artifacts/preview-death-replay-{0:00}-game.png' -f ($hero*4+$col))))
     try {
      $crop=[System.Drawing.Rectangle]::new([int]($source.Width*.33),[int]($source.Height*.23),[int]($source.Width*.34),[int]($source.Height*.55))
      $dest=[System.Drawing.Rectangle]::new($col*300+4,$row*310+26,292,280)
      $graphics.DrawImage($source,$dest,$crop,[System.Drawing.GraphicsUnit]::Pixel)
      $graphics.DrawString(($records[$hero].slug+' '+@(0,32,65,100)[$col]+'%'),$font,[System.Drawing.Brushes]::White,$col*300+8,$row*310+5)
     } finally {$source.Dispose()}
    }
   }
   $canvas.Save((Join-Path $projectRoot ('artifacts/death-replay/death-poses-{0}.png' -f ($sheet+1))),[System.Drawing.Imaging.ImageFormat]::Png)
  } finally {$graphics.Dispose();$canvas.Dispose()}
 }
} finally {$font.Dispose()}
