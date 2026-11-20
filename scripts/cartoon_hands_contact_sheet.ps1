# Arrange actual Blender renders without changing the source images.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$outputRoot = Join-Path (Split-Path -Parent $PSScriptRoot) 'artifacts/cartoon-hands'
$canvas = [System.Drawing.Bitmap]::new(1680, 738)
$graphics = [System.Drawing.Graphics]::FromImage($canvas)
$graphics.Clear([System.Drawing.ColorTranslator]::FromHtml('#17212d'))
$graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
$title = [System.Drawing.Font]::new('Malgun Gothic', 25)
$label = [System.Drawing.Font]::new('Malgun Gothic', 20)
$small = [System.Drawing.Font]::new('Malgun Gothic', 15)
$white = [System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml('#f2f5f9'))
$muted = [System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml('#afc6d9'))
try {
    $graphics.DrawString('손 모델 수정 · 에르핀 실제 3D 렌더', $title, $white, 30, 20)
    $columns = @(
        @('before', '수정 전', '5개 · 3마디'),
        @('open', '수정 후 — 펼친 손', '엄지 포함 4개 · 2마디'),
        @('grip', '수정 후 — 파지', '짧고 굵은 손가락 · 두 마디 굽힘')
    )
    for ($i = 0; $i -lt $columns.Count; $i++) {
        $x = $i * 560 + 20
        $graphics.DrawString($columns[$i][1], $label, $white, $x, 85)
        $graphics.DrawString($columns[$i][2], $small, $muted, $x, 125)
        $render = [System.Drawing.Image]::FromFile((Join-Path $outputRoot ('erpin-' + $columns[$i][0] + '-palm.png')))
        try { $graphics.DrawImage($render, $x, 167, 520, 520) } finally { $render.Dispose() }
    }
    $graphics.DrawString('같은 카메라·배율로 비교  |  엄지 포함 4개 · 손가락마다 2마디 · 짧고 둥근 실루엣', $small, $muted, 30, 704)
    $canvas.Save((Join-Path $outputRoot 'hand-comparison.png'), [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
    $graphics.Dispose(); $canvas.Dispose(); $title.Dispose(); $label.Dispose(); $small.Dispose(); $white.Dispose(); $muted.Dispose()
}
