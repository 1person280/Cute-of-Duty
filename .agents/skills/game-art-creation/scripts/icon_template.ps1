# 美术生成模板：复制到 tools/<内容名>.ps1 后修改【定制区】与几何绘制
# 运行：powershell -NoProfile -ExecutionPolicy Bypass -File tools\<内容名>.ps1
# 生成后用 Read 工具查看 PNG 自检（形状/透明/颜色是否对色板）
Add-Type -AssemblyName System.Drawing

# =====【定制区】=====
$out = Join-Path $env:TEMP 'icon_template_preview.png'   # 正式使用改为 ..\assets\ui\xxx.png
$size = 128
# —— 量化参数（数值规则见 references/style-guide.md「量化美学规范」，按画布短边换算）——
$bodyRatio  = 0.75   # 主体外径占画布 70–80%
$strokePct  = 0.031  # 外描边宽占画布 2.5–4%
$paddingPct = 0.10   # 内边距 ≥ 10%，主体不贴边
$cornerPct  = 0.12   # 圆角半径占画布 8–15%，可爱取上限
# 颜色必须取自 .agents/skills/game-art-creation/references/style-guide.md
# FromArgb(a, r, g, b)，RGB 按浮点值 ×255 四舍五入
$accent = [System.Drawing.Color]::FromArgb(255, 89, 204, 255)   # 冰 #59CCFF = (0.35, 0.80, 1.00)
$edge   = [System.Drawing.Color]::FromArgb(255, 31, 36, 46)     # UI 深底 #1F242E = (0.12, 0.14, 0.18)
# ====================

$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.Clear([System.Drawing.Color]::Transparent)

# —— 示例图形：圆形徽章（几何全部由量化参数推导，改数值即改风格）——
$c = $size / 2.0
$r = $size * $bodyRatio / 2.0
$stroke = [Math]::Max(2, [Math]::Round($size * $strokePct))
$brush = New-Object System.Drawing.SolidBrush($accent)
$pen = New-Object System.Drawing.Pen($edge, [float]$stroke)
$g.FillEllipse($brush, $c - $r, $c - $r, $r * 2, $r * 2)
$g.DrawEllipse($pen, $c - $r, $c - $r, $r * 2, $r * 2)

# —— 圆角矩形主体（需要方圆结合时启用，用 cornerPct / paddingPct）——
# $pad = $size * $paddingPct
# $rad = $size * $cornerPct
# $path = New-Object System.Drawing.Drawing2D.GraphicsPath
# $w = $size - $pad * 2; $h = $w
# $path.AddArc($pad, $pad, $rad * 2, $rad * 2, 180, 90)
# $path.AddArc($pad + $w - $rad * 2, $pad, $rad * 2, $rad * 2, 270, 90)
# $path.AddArc($pad + $w - $rad * 2, $pad + $h - $rad * 2, $rad * 2, $rad * 2, 0, 90)
# $path.AddArc($pad, $pad + $h - $rad * 2, $rad * 2, $rad * 2, 90, 90)
# $path.CloseFigure()
# $g.FillPath($brush, $path); $g.DrawPath($pen, $path); $path.Dispose()

# 在这里画你的主体图形（雪花/火焰/枪械剪影…）。
# 旋转对称图形的画法参考 tools/make_gear_icon.ps1：
#   $g.TranslateTransform($c, $c); $g.RotateTransform($角度) 后画矩形，循环 $g.ResetTransform()
$pen.Dispose(); $brush.Dispose()

$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Host ("saved: " + $out + " (" + (Get-Item $out).Length + " bytes)")
