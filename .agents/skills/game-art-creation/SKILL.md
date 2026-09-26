---
name: game-art-creation
description: Cute Of Duty 游戏美术创作指南——用 PowerShell + System.Drawing 程序化生成与游戏「3D 像素可爱风」一致的美术资产（图标/贴图/占位图），统一使用游戏内色板；角色建模遵循 [YSM] 是，史蒂夫模型 (Yes Steve Model) mod 的方块人美学。凡用户要求画图、生成图标、贴图、纹理、立绘、角色模型、建模（如"按史蒂夫模型美学做角色"）、UI 元素、配色/调色板，或为角色、武器、环境、UI 新增或修改美术资产时使用——即使用户没说"美术"二字，或只说"画一个""做个图标""这个颜色不对"。
---

# Cute Of Duty 美术创作

为 3D 像素风 FPS《Cute Of Duty》（Bevy 0.14）生成风格一致的美术资产。

## 第一步：配色必须对齐游戏内色板

本游戏的美术一致性靠**统一色板**维系——图标、UI、角色、环境共用同一套颜色，
源头是 `demo3d/src/main.rs` 里的 `Color::srgb` 值。生成任何美术前，先查完整色板
[references/style-guide.md](references/style-guide.md)。

最常用的核心色（浮点 sRGB 与代码完全一致）：

| 用途 | 颜色 | sRGB 浮点 | Hex |
|---|---|---|---|
| 火元素 | 橙红 | (1.00, 0.45, 0.12) | #FF731F |
| 冰元素 | 天蓝 | (0.35, 0.80, 1.00) | #59CCFF |
| 电元素 | 亮黄 | (0.95, 1.00, 0.15) | #F2FF26 |
| 毒元素 | 黄绿 | (0.50, 0.95, 0.20) | #80F233 |
| UI 强调 | 琥珀 | (1.00, 0.80, 0.25) | #FFCC40 |
| UI 深底 | 藏青 | (0.12, 0.14, 0.18) | #1F242E |

规则：

- 色板里有的颜色，不自造近似色；确需新颜色（新元素、新类别），先把色值登记进
  references/style-guide.md 再使用，保持单一事实来源。
- 改游戏内 3D/UI 颜色 = 改 `demo3d/src/main.rs` 的 Color::srgb；做贴图/图标 = 脚本里用同值。两边要同步，否则 UI 图标和游戏内弹道/特效会颜色打架。

**角色建模另有钦定标准：[YSM] 是，史蒂夫模型 (Yes Steve Model) mod 的方块人美学**——
在原版 Steve 方块骨架上加兽耳/发型/服装等特征方块、二次元皮肤级细节、Q 版比例、
可动部件动画。该 mod 的技术底座是 GeckoLib + 基岩版模型/动画 JSON（Blockbench 制作），
动画状态清单、贴图与包结构规范见 style-guide 的「格式与动画词汇」。注意是"这个 mod 的
美学"，**不是原版裸史蒂夫**。完整规范与参考图：
[references/style-guide.md](references/style-guide.md) 的「角色建模规范」+
steve-model-mod-reference.webp（YSM 的 Alt+Y 模型浏览器截图）。

## 第二步：用 PowerShell + System.Drawing 生成

本机**没有 Python、没有 Node、没有 ImageMagick/ffmpeg**，不要尝试这些。
已验证的美术生成路径是 PowerShell 的 System.Drawing——项目先例
`tools/make_gear_icon.ps1`（生成主菜单齿轮图标，已接入游戏）。

做法：

1. 复制 [scripts/icon_template.ps1](scripts/icon_template.ps1) 到项目 `tools/` 目录，
   按内容命名（如 `make_ice_rifle_icon.ps1`），然后改【定制区】的输出路径、颜色和
   几何绘制。模板已含标准骨架：透明底、抗锯齿、保存 PNG、输出文件大小。
   复杂几何（齿轮、旋转对称图形）的画法参考 `tools/make_gear_icon.ps1` 的
   TranslateTransform + RotateTransform 旋转画齿法。
2. 从 cmd 运行（注意：ZCode 的 Bash 工具是 cmd，不是 PowerShell）：

   ```
   powershell -NoProfile -ExecutionPolicy Bypass -File tools\make_ice_rifle_icon.ps1
   ```

**编码铁律：.ps1 带中文注释必须存成 UTF-8 with BOM。** Windows PowerShell 5.1 对无 BOM
文件按 ANSI/GBK 解析，中文注释乱码的字节可能吞掉相邻代码行——已实测事故：注释乱码吞掉
`$c = $size / 2.0` 赋值行，圆形画到画布左上角。ZCode 的 Write 工具写出的是无 BOM UTF-8，
所以写完 .ps1 后必须补 BOM：

```
powershell -NoProfile -Command "$p='tools\xxx.ps1'; $t=[System.IO.File]::ReadAllText($p,[System.Text.Encoding]::UTF8); [System.IO.File]::WriteAllText($p,$t,(New-Object System.Text.UTF8Encoding $true))"
```

补完 BOM 再运行生成，并按第四步目检。

**其余 PowerShell 陷阱（实测踩过）：**

- **变量名大小写不敏感**：`foreach ($b in $B)` 里 `$b` 与 `$B` 是同一个变量，循环结束后
  集合变量被覆盖成最后一个元素，后续代码只拿到一个坏项（实测：等轴测视图 34 个盒子只
  画出 1 个）。集合与循环变量必须用不同名字（如 `$Boxes` / `$box`）。
- **逗号优先级高于算术**：`@($x + 8, 60)` 被解析成 `$x + (8,60)` 而报 op_Addition 错；
  数组成员里的算术表达式要加括号：`@(($x + 8), 60)`。
- **函数第一参传负数字面量**会被当成参数名（`PX -4 32 -4` 报错）；用变量传参。

## 第三步：落盘与命名

| 目录 | 内容 |
|---|---|
| assets/ui/ | HUD、菜单、图标（代码引用形如 `assets.load("ui/xxx.png")`） |
| assets/weapons/ | 武器 |
| assets/characters/ | 角色 |
| assets/environment/ | 环境 |
| assets/elements/ | 元素相关（当前为空） |

- 文件名全小写 snake_case；UI/物品图标统一 128×128。
- 需要透明的图（图标、HUD 元素）**必须 PNG**——JPG 没有透明通道，现有 .jpg 全是方形概念图。
- 游戏内代码引用路径相对 `assets/`，如 `assets.load("ui/gear_icon.png")`，见 demo3d/src/main.rs:464。
- 注意：assets/ 下现有 .jpg（角色/武器/环境/element_icons）是早期 AI 概念图，
  **没有任何代码引用**。要把美术真正接进游戏：用 PNG，并在 demo3d/src/main.rs 里
  `assets.load()` 加载（仅 demo 壳持有 bevy，核心库 src/ 永远不引用资产）。

## 第四步：自检后再交付

生成完必须用 Read 工具打开 PNG 亲眼检查——形状是否可辨、透明是否生效、
颜色是否落在色板上——再向用户汇报。汇报包含：文件路径、尺寸、用了哪些色板值。

## Bevy 0.14 集成陷阱（把美术接进游戏时）

- 新增元素配色要同步改 demo3d/src/main.rs 的 `ElementType::color()`。
- 特效网格/材质必须入池共享（MaterialPool），逐发新建会复现已修复过的爆炸 OOM。
- UI 默认用 NodeBundle + BackgroundColor 纯色而不是贴图；只有图标类才需要 Image 资产。
- `Color::srgb(r, g, b)` 收 0~1 浮点；半透明用 `Color::srgba(..., a)` 或 `.with_alpha(a)`。
- 中文字体用 assets/simhei.ttf。
