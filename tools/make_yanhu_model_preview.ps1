# 生成焰狐 voxel 建模预览图（自检用）：左=正视(看 -Z 脸)，右=等轴测
#
# 设计动机（Why）：预览必须**直接解析** ServerCode/model/FireFox.json，
# 而不是另写一份盒表——后者会与造型源文件漂移（旧版脚本即因硬编码盒表、且输出到已删除的
# HostCode/assets 而失效）。盒色按 JSON 的 `mat` 逐盒取，与客户端 material_color 同口径。
#
# 用法：powershell -NoProfile -ExecutionPolicy Bypass -File tools\make_yanhu_model_preview.ps1 [-Out <png路径>]
param(
    [string]$Out = (Join-Path $env:TEMP 'yanhu_model_preview.png')
)

Add-Type -AssemblyName System.Drawing

$geoPath = Join-Path $PSScriptRoot '..\ServerCode\model\FireFox.json'
$doc = Get-Content -Raw -Encoding UTF8 $geoPath | ConvertFrom-Json
# 顶层键名含冒号，用 PSObject.Properties 取，避免属性访问语法歧义。
$geoms = $doc.PSObject.Properties['minecraft:geometry'].Value
$bones = $geoms[0].bones

# 材质键 -> 十六进制（与 HostCode/world/voxel_model.rs::material_color 一致）
$Mat = @{
    skin      = 'F5CCB0'; green  = '385238'; dark      = '262B26'
    armor     = '737A80'; boot   = '2E241F'; hair      = 'C25C1F'
    hair_dark = '8C3D17'; cream  = 'F5E0C7'; eye       = '4DE6FF'
    gun       = '4D4D59'; accent = 'FF731F'
}

# 拍平为盒列表（预览按静置姿态、忽略骨旋转，尾巴等小角度件的差异可接受）
$Boxes = @()
foreach ($bone in $bones) {
    foreach ($cube in $bone.cubes) {
        $o = $cube.origin; $s = $cube.size
        $key = "$($cube.mat)"
        if (-not $Mat.ContainsKey($key)) { $key = 'dark' }
        $Boxes += @{ x = [double]$o[0]; y = [double]$o[1]; z = [double]$o[2]
                     w = [double]$s[0]; h = [double]$s[1]; d = [double]$s[2]
                     c = $Mat[$key] }
    }
}

$W = 1080; $H = 640
$bmp = New-Object System.Drawing.Bitmap($W, $H)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.Clear([System.Drawing.Color]::FromArgb(255, 31, 36, 46))   # UI 深底 #1F242E

function Shade($hex, $f) {
    $r = [Math]::Min(255, [int]([Convert]::ToInt32($hex.Substring(0,2),16) * $f))
    $gg = [Math]::Min(255, [int]([Convert]::ToInt32($hex.Substring(2,2),16) * $f))
    $b = [Math]::Min(255, [int]([Convert]::ToInt32($hex.Substring(4,2),16) * $f))
    return [System.Drawing.Color]::FromArgb(255, $r, $gg, $b)
}
function BrushOf($c, $f) { return New-Object System.Drawing.SolidBrush((Shade $c $f)) }

$font = New-Object System.Drawing.Font('Consolas', 13)
$lbl = BrushOf '8C9EB8' 1.0
$g.DrawString('YAN HU  (Yes Steve Model style voxel spec, unit = design px)', $font, $lbl, 20, 12)

# ============ 左:正视(看 -Z 脸) ============
$s1 = 11; $ox1 = 225; $gy1 = 600
$g.DrawString('FRONT (-Z)', $font, $lbl, 150, 40)
$penG = New-Object System.Drawing.Pen((Shade '4A5464' 1.0), 2)
$g.DrawLine($penG, $ox1 - 9.5*$s1, $gy1, $ox1 + 9.5*$s1, $gy1)
foreach ($box in ($Boxes | Sort-Object { -($_.z) })) {
    $br = BrushOf $box.c 1.0
    $pen = New-Object System.Drawing.Pen((Shade $box.c 0.55), 1)
    $rx = $ox1 + $box.x * $s1; $ry = $gy1 - ($box.y + $box.h) * $s1
    $g.FillRectangle($br, $rx, $ry, $box.w * $s1, $box.h * $s1)
    $g.DrawRectangle($pen, $rx, $ry, $box.w * $s1, $box.h * $s1)
    $br.Dispose(); $pen.Dispose()
}

# ============ 右:等轴测(视点在右前上) ============
$s2 = 8.4; $ox2 = 705; $oy2 = 380
$g.DrawString('ISO (viewer at +X, -Z)', $font, $lbl, 790, 40)
$sh = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(60, 0, 0, 0))
$g.FillEllipse($sh, $ox2 - 105, $oy2 + 8, 210, 60)
$sh.Dispose()
function PX([double]$x, [double]$y, [double]$z) {
    $sx = $ox2 + ($x + $z) * 0.866 * $s2
    $sy = $oy2 + ($x - $z) * 0.5 * $s2 - $y * $s2
    return [System.Drawing.PointF]::new([single]$sx, [single]$sy)
}
function Quad([System.Drawing.SolidBrush]$brush, $p1, $p2, $p3, $p4) {
    $script:g.FillPolygon($brush, [System.Drawing.PointF[]]@($p1, $p2, $p3, $p4))
}
# 近 = (x - z) 大 → 远的先画
$sorted = @($Boxes) | Sort-Object -Property @{Expression={ [double]($_.x - $_.z) }}, @{Expression={ [double]$_.y }}
foreach ($box in $sorted) {
    $x = $box.x; $y = $box.y; $z = $box.z
    $mx = $x + $box.w; $my = $y + $box.h; $mz = $z + $box.d
    $brL = BrushOf $box.c 0.92; $brR = BrushOf $box.c 0.70; $brT = BrushOf $box.c 1.15
    Quad $brT (PX $x $my $z)  (PX $mx $my $z)  (PX $mx $my $mz) (PX $x $my $mz)
    Quad $brL (PX $x $y $z)   (PX $mx $y $z)   (PX $mx $my $z)  (PX $x $my $z)
    Quad $brR (PX $mx $y $z)  (PX $mx $y $mz)  (PX $mx $my $mz) (PX $mx $my $z)
    $brL.Dispose(); $brR.Dispose(); $brT.Dispose()
}

$outDir = Split-Path $Out
if ($outDir -and -not (Test-Path $outDir)) { New-Item -ItemType Directory -Path $outDir | Out-Null }
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Host ("saved: " + $Out + " (" + (Get-Item $Out).Length + " bytes)")