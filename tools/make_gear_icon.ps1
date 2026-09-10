# 生成主菜单齿轮图标：assets/ui/gear_icon.png（透明底、浅灰齿轮、中心镂空）
Add-Type -AssemblyName System.Drawing
$size = 128
$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.Clear([System.Drawing.Color]::Transparent)
$brush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 216, 226, 236))
$c = 64.0; $bodyR = 42.0; $outerR = 58.0; $holeR = 20.0; $teeth = 8; $toothW = 17.0
$g.FillEllipse($brush, $c - $bodyR, $c - $bodyR, $bodyR * 2, $bodyR * 2)
for ($i = 0; $i -lt $teeth; $i++) {
    $g.ResetTransform()
    $g.TranslateTransform($c, $c)
    $g.RotateTransform($i * 360 / $teeth)
    $g.FillRectangle($brush, -$toothW / 2, -$outerR, $toothW, $outerR - $bodyR + 2)
}
$g.ResetTransform()
$g.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
$holeBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::Transparent)
$g.FillEllipse($holeBrush, $c - $holeR, $c - $holeR, $holeR * 2, $holeR * 2)
$out = Join-Path $PSScriptRoot '..\assets\ui\gear_icon.png'
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Host ("saved: " + (Get-Item $out).Length + " bytes")
