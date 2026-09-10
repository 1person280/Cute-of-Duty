# 美术生成模板：复制到 tools/<内容名>.ps1 后修改【定制区】与几何绘制
# 运行：powershell -NoProfile -ExecutionPolicy Bypass -File tools\<内容名>.ps1
# 生成后用 Read 工具查看 PNG 自检（形状/透明/颜色是否对色板）
Add-Type -AssemblyName System.Drawing

# =====【定制区】=====
$out = Join-Path $env:TEMP 'icon_template_preview.png'   # 正式使用改为 ..\assets\ui\xxx.png
$size = 128
# 颜色必须取自 .agents/skills/game-art-creation/references/style-guide.md
# FromArgb(a, r, g, b)，RGB 按浮点值 ×255 四舍五入
$accent = [System.Drawing.Color]::FromArgb(255, 89, 204, 255)   # 冰 #59CCFF = (0.35, 0.80, 1.00)
$edge   = [System.Drawing.Color]::FromArgb(255, 31, 36, 46)     # UI 深底 #1F242E = (0.12, 0.14, 0.18)
# ====================

$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.Clear([System.Drawing.Color]::Transparent)

# —— 示例图形：圆形徽章（主体占画布约 75%，居中）——
$c = $size / 2.0
$r = $size * 0.375
$brush = New-Object System.Drawing.SolidBrush($accent)
$pen = New-Object System.Drawing.Pen($edge, 4)
$g.FillEllipse($brush, $c - $r, $c - $r, $r * 2, $r * 2)
$g.DrawEllipse($pen, $c - $r, $c - $r, $r * 2, $r * 2)

# 在这里画你的主体图形（雪花/火焰/枪械剪影…）。
# 旋转对称图形的画法参考 tools/make_gear_icon.ps1：
#   $g.TranslateTransform($c, $c); $g.RotateTransform($角度) 后画矩形，循环 $g.ResetTransform()
$pen.Dispose(); $brush.Dispose()

$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Host ("saved: " + $out + " (" + (Get-Item $out).Length + " bytes)")
