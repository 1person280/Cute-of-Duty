# 生成焰狐 voxel 建模预览图 -> assets/characters/yanhu_model_preview.png
# 左:正视 图 右:等轴测;盒子表即建模规格(与 yanhu.geometry.json 一致)
Add-Type -AssemblyName System.Drawing

$out = Join-Path $PSScriptRoot '..\assets\characters\yanhu_model_preview.png'
$W = 1080; $H = 640
$bmp = New-Object System.Drawing.Bitmap($W, $H)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.Clear([System.Drawing.Color]::FromArgb(255, 31, 36, 46))   # UI 深底 #1F242E

# --- 盒子表:x,z 相中心;y 向上;单位=设计像素(头8=1世界单位);c 为十六进制色 ---
# 注意:集合名避开单字母,防与循环变量撞名(PowerShell 变量名大小写不敏感!)
$Boxes = @(
    @{n='bootL';   x=-4;  y=0;  z=-3;   w=4;  h=3;  d=5;   c='2E241F'},
    @{n='bootR';   x=0;   y=0;  z=-3;   w=4;  h=3;  d=5;   c='2E241F'},
    @{n='legL';    x=-4;  y=3;  z=-2;   w=4;  h=9;  d=4;   c='385238'},
    @{n='legR';    x=0;   y=3;  z=-2;   w=4;  h=9;  d=4;   c='385238'},
    @{n='torso';   x=-4;  y=12; z=-2;   w=8;  h=12; d=4;   c='385238'},
    @{n='belt';    x=-4;  y=12; z=-2.5; w=8;  h=2;  d=5;   c='262B26'},
    @{n='vest';    x=-4;  y=15; z=-3;   w=8;  h=5;  d=1;   c='262B26'},
    @{n='vestPin'; x=-1;  y=16; z=-3.3; w=2;  h=2;  d=0.3; c='FF731F'},
    @{n='shdL';    x=-6;  y=22; z=-3;   w=3;  h=2;  d=3;   c='737A80'},
    @{n='shdR';    x=3;   y=22; z=-3;   w=3;  h=2;  d=3;   c='737A80'},
    @{n='armLup';  x=-8;  y=19; z=-2;   w=4;  h=5;  d=4;   c='385238'},
    @{n='armLlo';  x=-8;  y=12; z=-2;   w=4;  h=7;  d=4;   c='F5CCB0'},
    @{n='armRup';  x=4;   y=19; z=-2;   w=4;  h=5;  d=4;   c='385238'},
    @{n='armband'; x=3.9; y=20; z=-2.1; w=4.2;h=2;  d=4.2; c='FF731F'},
    @{n='armRlo';  x=4;   y=12; z=-2;   w=4;  h=7;  d=4;   c='F5CCB0'},
    @{n='gunBody'; x=4.2; y=16; z=-6;   w=1;  h=1;  d=7;   c='4D4D59'},
    @{n='gunMag';  x=4.3; y=14.5;z=-3;  w=0.7;h=1.5;d=2;   c='262B26'},
    @{n='gunStock';x=4.3; y=15.8;z=0;   w=0.7;h=0.8;d=2;   c='262B26'},
    @{n='gunSight';x=4.3; y=17;  z=-3;  w=0.4;h=1;  d=1;   c='FF731F'},
    @{n='head';    x=-4;  y=24; z=-4;   w=8;  h=8;  d=8;   c='F5CCB0'},
    @{n='eyeL';    x=-3;  y=27; z=-4.3; w=2;  h=2;  d=0.3; c='FFCC40'},
    @{n='eyeR';    x=1;   y=27; z=-4.3; w=2;  h=2;  d=0.3; c='FFCC40'},
    @{n='hairCap'; x=-4.5;y=29; z=-4.5; w=9;  h=3;  d=9;   c='C25C1F'},
    @{n='hairBack';x=-4.5;y=24; z=3.5;  w=9;  h=6;  d=1;   c='8C3D17'},
    @{n='hairSideL';x=-5.5;y=24;z=-4;   w=1;  h=5;  d=8;   c='8C3D17'},
    @{n='hairSideR';x=4.5;y=24; z=-4;   w=1;  h=5;  d=8;   c='8C3D17'},
    @{n='bangs';   x=-4;  y=27; z=-5;   w=8;  h=3;  d=0.5; c='C25C1F'},
    @{n='streak';  x=-2;  y=29; z=-5.3; w=2;  h=2;  d=0.3; c='FF731F'},
    @{n='earL';    x=-4;  y=32; z=-2;   w=3;  h=3;  d=2;   c='C25C1F'},
    @{n='earLin';  x=-3.4;y=32.6;z=-2.3;w=1.8;h=1.8;d=0.3; c='F5E0C7'},
    @{n='earR';    x=1;   y=32; z=-2;   w=3;  h=3;  d=2;   c='C25C1F'},
    @{n='earRin';  x=1.6; y=32.6;z=-2.3;w=1.8;h=1.8;d=0.3; c='F5E0C7'},
    @{n='tailA';   x=-1.5;y=13; z=2;    w=3;  h=3;  d=7;   c='C25C1F'},
    @{n='tailB';   x=-1;  y=15; z=8;    w=2;  h=2;  d=6;   c='C25C1F'},
    @{n='tailC';   x=-1;  y=16; z=13;   w=2;  h=2;  d=4;   c='F5E0C7'}
)

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
# 地面线
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

# ============ 右:等轴测(看 +X 与 -Z,视点在右前上) ============
$s2 = 8.4; $ox2 = 705; $oy2 = 380
$g.DrawString('ISO (viewer at +X, -Z)', $font, $lbl, 790, 40)
# 地面阴影
$sh = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(60, 0, 0, 0))
$g.FillEllipse($sh, $ox2 - 105, $oy2 + 8, 210, 60)
$sh.Dispose()
function PX([double]$x, [double]$y, [double]$z) {
    $sx = $ox2 + ($x + $z) * 0.866 * $s2
    $sy = $oy2 + ($x - $z) * 0.5 * $s2 - $y * $s2
    return [System.Drawing.PointF]::new([single]$sx, [single]$sy)
}
function Quad([System.Drawing.SolidBrush]$brush, [System.Drawing.PointF]$p1, [System.Drawing.PointF]$p2, [System.Drawing.PointF]$p3, [System.Drawing.PointF]$p4) {
    $script:g.FillPolygon($brush, [System.Drawing.PointF[]]@($p1, $p2, $p3, $p4))
}
# 近 = (x - z) 大 → 远的先画:(x - z) 升序,同层 y 升序
$sorted = @($Boxes) | Sort-Object -Property @{Expression={ [double]($_.x - $_.z) }}, @{Expression={ [double]$_.y }}
foreach ($box in $sorted) {
    $x = [double]$box.x; $y = [double]$box.y; $z = [double]$box.z
    $mx = $x + [double]$box.w; $my = $y + [double]$box.h; $mz = $z + [double]$box.d
    $brL = BrushOf $box.c 0.92; $brR = BrushOf $box.c 0.70; $brT = BrushOf $box.c 1.15
    Quad $brT (PX $x $my $z)  (PX $mx $my $z)  (PX $mx $my $mz) (PX $x $my $mz)   # top
    Quad $brL (PX $x $y $z)   (PX $mx $y $z)   (PX $mx $my $z)  (PX $x $my $z)    # front (-Z)
    Quad $brR (PX $mx $y $z)  (PX $mx $y $mz)  (PX $mx $my $mz) (PX $mx $my $z)   # right (+X)
    $brL.Dispose(); $brR.Dispose(); $brT.Dispose()
}

$outDir = Split-Path $out
if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Path $outDir | Out-Null }
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Host ("saved: " + $out + " (" + (Get-Item $out).Length + " bytes)")
