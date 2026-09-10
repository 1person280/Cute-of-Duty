# 生成焰狐干员头像图标 128x128 -> assets/ui/yanhu_avatar.png
# 色板取自 .agents/skills/game-art-creation/references/style-guide.md(含焰狐专属色)
Add-Type -AssemblyName System.Drawing

$out = Join-Path $PSScriptRoot '..\assets\ui\yanhu_avatar.png'
$size = 128
$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.Clear([System.Drawing.Color]::Transparent)

function Brush($hex) {
    return New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(
        [Convert]::ToInt32($hex.Substring(0,2),16),
        [Convert]::ToInt32($hex.Substring(2,2),16),
        [Convert]::ToInt32($hex.Substring(4,2),16)))
}
$fire  = Brush 'FF731F'   # 火元素
$field = Brush '243347'   # 面板底
$hair  = Brush 'C25C1F'   # 焰狐发色
$hairD = Brush '8C3D17'   # 发色暗部
$cream = Brush 'F5E0C7'   # 毛尖米白
$skin  = Brush 'F5CCB0'   # 肤色
$amber = Brush 'FFCC40'   # 琥珀(眼)
$dark  = Brush '262B26'   # 作战服暗部
$green = Brush '385238'   # 作战服
$white = Brush 'EBF2FF'   # 眼高光

function Ell($brush, $cx, $cy, $rx, $ry) {
    $script:g.FillEllipse($brush, $cx - $rx, $cy - $ry, $rx * 2, $ry * 2)
}
function Poly($brush, $pts) {
    $pf = [System.Drawing.PointF[]]::new($pts.Count)
    for ($i = 0; $i -lt $pts.Count; $i++) { $pf[$i] = New-Object System.Drawing.PointF($pts[$i][0], $pts[$i][1]) }
    $script:g.FillPolygon($brush, $pf)
}

# --- 徽章底:内圆面板底 ---
Ell $field 64 66 52 52

# --- 主体裁剪到 r=57 圆内 ---
$path = New-Object System.Drawing.Drawing2D.GraphicsPath
$path.AddEllipse(64 - 57, 66 - 57, 114, 114)
$g.SetClip($path)

# --- 狐耳(左右,耳尖出血到环内) ---
Poly $hair  @( @(38,44), @(32,8),  @(58,30) )          # 左耳外层
Poly $cream @( @(39,38), @(35,15), @(52,29) )          # 左耳内耳
Poly $hair  @( @(90,44), @(96,8),  @(70,30) )          # 右耳外层
Poly $cream @( @(89,38), @(93,15), @(76,29) )          # 右耳内耳

# --- 后发:大团 + 两侧垂发 ---
Ell $hair 64 60 37 36
Ell $hairD 30 76 8 16
Ell $hairD 98 76 8 16

# --- 脸 ---
Ell $skin 64 76 23 21

# --- 刘海:一排下垂三角 + 发带 ---
$g.FillRectangle($hair, 41, 52, 46, 10)
for ($x = 41; $x -lt 87; $x += 8) {
    Poly $hair @( @($x, 60), @(($x + 8), 60), @(($x + 4), 69) )
}
Poly $fire @( @(58, 55), @(62, 55), @(59, 66) )        # 火色挑染
Poly $fire @( @(72, 55), @(76, 55), @(75, 64) )        # 火色挑染

# --- 眼睛(琥珀 + 高光) ---
$g.FillRectangle($amber, 48, 72, 8, 10)
$g.FillRectangle($amber, 72, 72, 8, 10)
$g.FillRectangle($white, 50, 74, 3, 3)
$g.FillRectangle($white, 74, 74, 3, 3)

# --- 腮红(半透明火色) ---
$blush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(90, 255, 115, 31))
Ell $blush 44 84 6 3
Ell $blush 84 84 6 3

# --- 领口(作战服 + 火色徽标) ---
Ell $dark 64 122 30 20
$g.FillRectangle($green, 38, 100, 52, 6)
$g.FillRectangle($fire, 60, 108, 8, 8)

$g.ResetClip()

# --- 徽章环(压在主体上收边) ---
$penRing = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(255, 255, 115, 31), 7)
$g.DrawEllipse($penRing, 64 - 55, 66 - 55, 110, 110)
$penRing.Dispose()

$outDir = Split-Path $out
if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Path $outDir | Out-Null }
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Host ("saved: " + $out + " (" + (Get-Item $out).Length + " bytes)")
