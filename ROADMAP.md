# Cute Of Duty — 底层更新路线图（0.4）

> 本次版本以「底层更新」为专项：升级全部底层依赖，核心是 **Bevy 0.14 → 0.19**（跨 5 大版本）。
> 完成批次后版本升至 **0.5.0**。
>
> 原则：分批推进、每批编译验证；保留核心库业务逻辑零改动；push 到 GitHub 远端即为每批验收。

## 一、全底层依赖盘点与升级优先级

| 依赖 | 当前 | 目标 | 优先级 | 说明 |
|---|---|---|---|---|
| **bevy** | 0.14 | **0.19** | P0 | 本次主角，跨 5 大版本，单独一个专项分批 |
| tokio | 1.35 | 1.4x 最新 | P1 | 主循环节流；小版本连升，低风险 |
| serde_yaml | 0.9 | 0.9 最新补丁 | P1 | 配置解析；同 minor 补丁，低风险 |
| rand / rand_pcg | 0.8 / 0.3 | 0.8.x 最新 | P2 | 战术随机；0.9 有 major 变化，暂缓 |
| serde | 1.0 | 1.x 最新 | P2 | 生态跟随即可 |
| crossbeam-queue | 0.3 | 0.3 最新 | P3 | HAL 环形缓冲；稳定 |
| blake3 | 1.5 | 1.x 最新 | P3 | 审计哈希；稳定 |
| tracing / tracing-subscriber | 0.1 / 0.3 | 最新补丁 | P3 | 日志；跟随 |
| criterion(dev) | 0.5 | 0.5 最新 | P3 | 仅 benchmark 用，无风险 |

**升级原则（三条线互不打扰）**
1. P1 依赖可独立先升，核心库 `cargo test` 秒级验证；
2. P0 Bevy 单独一个专项，经本路线图批准后分批走；
3. P2/P3 低价值小依赖随版本收尾顺带升，不单开批次。

## 二、Bevy 0.14 → 0.19 分批迁移

### 批 次

| 批次 | 范围 | 涉及文件 |
|---|---|---|
| M0 | 依赖复位：升 bevy 0.19，建立编译基线 | Cargo.toml |
| M1 | 渲染实体 Bundle 迁移（PbrBundle/Camera/灯/Spatial/Mesh） | src/model/*、src/demo/{world,character,controller,camera,loadout,supply_crate,stations,targets}.rs、src/demo/combat/* |
| M2 | 文本/CJK 字体专项（Parley TextFont/TextColor/FontSource） | src/demo/frontend.rs + 全部 Text 用法 |
| M3 | UI 面板迁移（Node/Style、GlobalZIndex、ImageNode） | src/demo/{menu,hud,inventory}/*、minimap.rs、bigmap.rs |
| M4 | ECS/系统完善（.single() Result、despawn、光标方法化、Assets::insert） | 全体 demo/model |
| M5 | 收敛与回归 → 版本升 0.5.0 | 全量检查 + 手动跑 demo |

### 技术对照表（0.14 → 0.19）

| 0.14 旧写法 | 0.19 新写法 |
|---|---|
| `PbrBundle { mesh, material, transform, .. }` | `(Mesh3d(mesh), MeshMaterial3d(mat), Transform::from_xyz(..))` |
| `Camera3dBundle::default()` | `Camera3d`（required 自动补） |
| `Camera2dBundle::default()` | `Camera2d` |
| `SpatialBundle { transform, ..default() }` | `Transform::from_xyz(..)` |
| `DirectionalLightBundle`/`PointLightBundle` | `(DirectionalLight/PointLight {..}, Transform)` |
| `TextBundle::from_section`/`Text::from_section(s, TextStyle{..})` | `(Text::new(s), TextFont{..}, TextColor(..), TextLayout::justify(..))` |
| `TextStyle { font_size: f32, color, .. }` | `TextFont { font_size: FontSize::Px(n), .. }` + 独立 `TextColor` |
| `NodeBundle { style: Style{..}, ... }` | `(Node { 布局字段 }, BackgroundColor, BorderColor, BorderRadius)` |
| `ZIndex::Global(n)` | `GlobalZIndex(n)` |
| `UiImage::new(handle)` | `UiImage::new`/`ImageNode`（以编译为准） |
| `.with_children(\|c: &mut ChildBuilder\| ..)` | 删类型注解：`.with_children(\|c\| ..)` |
| `.despawn_recursive()` | `.despawn()` |
| `.single()`（0.14 panic） | `.single()` 返回 `Result`，需解包 |
| `bevy::pbr::NotShadowCaster` | `bevy::light::NotShadowCaster` |
| `Handle<Font>` 默认覆盖（CJK） | 失效 → 每个 `TextFont.font: FontSource::from(cjk)` |
| `fonts.insert(id, font)` | 0.17 起返回 `Result`，`let _ = fonts.insert(id, font);` |
| `window.cursor.visible/grab_mode = ..` | `window.set_cursor_visible(bool)`/`window.grab_cursor(CursorGrabMode)` |

## 三、风险与回退

- Bevy 编译错误量大（45 文件 / 68 use），编译驱动逐波修复为预期。
- 最高风险：Parley 文本 + CJK 字体、`.single()` 改 Result、UiImage/ImageNode 命名、Resources-as-components broad query 冲突。
- 若个别 API 与编译结果不符，以编译器报错为准修正，不回退 bevy 版本。
- push 如遇网络中断（历史 `Recv failure`）：失败重试直至 `git status` 显示同步成功。