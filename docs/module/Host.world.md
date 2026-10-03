# Host.world · module.md

> 客户端 3D 世界子模块（`HostCode/world/`）。分层定位：**表现层 · 3D 场景与相机**。训练场几何、相机枢轴、体素造型均为表现层装配；实体身份（干员型号）与位置、血量等一律服务端权威，本模块只按快照渲染。

## 边界

**本模块负责**：
- **相机**：`camera`——第三人称越肩 SpringArm（`ChaseCamera`）、鼠标自由视角（`mouse_look_system` 写 `AimRig`）、跟随与避障（`follow_system`，射线-AABB 扫掠 + 非对称指数平滑）、持雷强制越肩（`sync_grenade_aim`）。
- **场景**：`scene`——基础装配（相机 / 共享 `CubeMesh` / 环境光，`Startup` 一次性）与几何装配（`spawn_world_when_ready` 消费 `WorldCatalog.layout`，就绪首帧生成光照 + 静态层 floor/props/glows；`targets`/`pickups` 不重复生成）。
- **体素造型**：`voxel`——`model`（`VoxelMaterials` + `spawn_voxel_body`）、`idle`（`drive_idle` 按服务端动画表达式写枢轴旋转）、`facing`（`face_aim_direction` 对齐视线偏航）。
- **干员配色预设**：`model`（`voxel_for` / `tint_code` / `tint_colors`）。
- **手雷预览**：`grenade/preview`（`draw_grenade_preview`，与服务端同源弹道常数）。

**本模块不负责**（明确划出）：
- **不做**位置/身份权威——坐标取自 `SnapshotBuffer`，模型身份取自快照 `model_preset`。
- **不定义**动态实体——靶/拾取物/玩家一律经 `net::snapshot` 对账生成，本模块绝不重复生成。
- **不持**世界目录真相——`WorldCatalog` 归 `flow`（服务端下发副本）。

## 数据所有权

| 数据 | 类型 | 是否可变 | 谁可写 |
|---|---|---|---|
| 越肩相机 | `ChaseCamera` | 每帧重写 | 仅 `world::camera`（`follow_system`） |
| 体素材质缓存 | `VoxelMaterials` | 只增复用 | 仅 `world::voxel` |
| 场景实体（地板/props/glows） | `Component` 集 | 生成期 | 仅 `world::scene`（就绪首帧一次） |
| 视角姿态 | `AimRig` | 可变 | 写者 `world::camera`；读者 `net::input_system` / 相机 |
| 世界目录（只读） | `Res<WorldCatalog>` | — | 只读（所有者 `flow`） |

> **所有权原则**：相机与场景几何归 `world`；`AimRig` 是"相机取景 = 上行瞄准"的同源单点，写入收敛到 `world::camera`。

## 接口

| 接口 | 形式 | 契约 | 兼容性承诺 |
|---|---|---|---|
| `spawn_scene_baseline` | 系统 | `Startup`：相机 + `CubeMesh` + 环境光 | 早于 `Update` 系统，资源硬依赖 |
| `spawn_world_when_ready` | 系统 | 布局就绪首帧生成光照 + 静态层 | 幂等（`Local` 守卫） |
| `spawn_camera` / `follow_system` / `mouse_look_system` / `sync_grenade_aim` | 系统 | 相机装配 / 跟随 / 视角 / 持雷叠加 | `mouse_look` 受 `gameplay_input_active` 门控 |
| `drive_idle` / `face_aim_direction` | 系统 | 体素 idle 动画 / 朝向对齐 | 消费 `ModelCatalog` |
| `draw_grenade_preview` | 系统 | 手雷预测抛物线 | 与服务端弹道常数同源 |

## 事件

| 事件名 | 方向 | 载荷契约 | 订阅者 |
|---|---|---|---|
| `MouseMotion`（消费） | Bevy 输入 → `world` | 鼠标位移 | `mouse_look_system` |
| —（本模块不产生跨模块事件） | — | — | — |

## 成熟度

**L1（内部稳定：crate 内可依赖）** —— 依据：`world` 是 3D 表现的完整实现，被 `launcher` 接线；不定义对外线格式。

- 破坏性变更纪律：须写 ADR + 1 名 reviewer。
- 已发生示例：镜头避障由"单条射线瞬时跳变"改为非对称指数平滑（收缩 25/s、回伸 10/s，[barek-history](../barek-history.md) 见物库修复条目）。
