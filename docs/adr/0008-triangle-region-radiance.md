# ADR 0008 · 三角形区域光线追踪着色器套件（并入 Bevy/wgpu · WGSL 单源）

- **状态**：已接受（Accepted）· **计划中（未实现）**
- **日期**：2026-10-02
- **决策者**：项目 owner
- **影响范围**：`HostCode`（新增顶层模块 `shader/`）、`ServerCode`（新增子域 `scene/`、`net/stages.rs` 编排）、`ContractCode`（新增 `scene.rs` 线格式类型、`net/protocol.rs` 成对消息、`port.rs` `SceneSource`）、`docs/contracts/protocol.yaml`、`docs/barek-history.md`
- **依据**：[ADR 0001](0001-modular-monolith-event-bus.md)「服务端权威」铁律；[ADR 0003](0003-contract-crate.md) 契约 crate 与服务端权威；[ADR 0004](0004-client-layer-convergence.md) 表现层只消费快照；[ADR 0006](0006-small-fixed-packet-dual-channel.md) 256B/4096B 双通道线格式；[模块边界](../architecture/module-boundaries.md) 分层纪律（`HostCode` 不得依赖 `ServerCode`）
- **产出文档**：[计划 0001 · 区域光照着色器套件](../plans/0001-区域光照着色器套件.md)

---

## 背景（Context）

需求是"以**三角形区域**为光照计算单位（而非逐像素），在不依赖 DXR / Vulkan Ray Tracing / RT Core 的前提下，
用极低算力产出像素风的区域阴影 / 区域 AO / 低精度反射 / 简单间接光"。

原始范式要求"**SPIR-V 1.0 为中间表示 + Vulkan / OpenGL / DX11 / DX12 / Metal 五后端**"。但本项目客户端是 **Bevy 0.15**，
其渲染后端是 **wgpu**：着色器源为 **WGSL**，后端覆盖 **Vulkan 1.x / OpenGL(ES) / DX12 / Metal**，**不支持 DX11**。

若照抄"手写五后端"，等于在一个 Bevy 客户端内再造一个渲染引擎：代码量远超"几千行"预算，且与
「单文件 ≤ 600 行 / 无环依赖 / 禁跨模块直调」三条仓库红线直接冲突。

同时，几何数据的所有权必须裁决：SceneGrid 是渲染加速结构，但仓库铁律要求"服务端权威、客户端只画"。

## 决策（Decision）

### 决策 1 · 后端收敛为 WGSL 单源，DX11 显式剔除

- 着色器**只写 WGSL**；后端由 **wgpu** 提供（Vulkan 1.x / GL(ES) / DX12 / Metal 免费获得）。
- **不手写** SPIR-V / GLSL / HLSL / MSL 后端；**不实现 DX11**（wgpu 不支持，列为范式不支持项）。
- 如未来确需其它 IR/后端，走 **naga 转译**（从 WGSL 派生），而非手写。
- 新增子模块 `shader/backend.rs` 作为**唯一**接触 wgpu / Bevy 渲染图的地方；算法模块（`grid/region/trace/shade/raster`）与后端无关。

### 决策 2 · 几何由服务端切分并下发，客户端按需拉取

- 服务端新增子域 `ServerCode/scene/`（`unit.rs` 切分 + 密度约束、`index.rs` 构建 + 查询），由契约静态布局 `MapLayout` 展平成"每 1m³ 单元三角面汤"。
- 契约新增**成对消息**：上行 `ClientMessage::RequestSceneUnits { coords }`、下行 `ServerMessage::SceneUnits { units }`；只传面汤（`TriangleData`），索引结构由客户端重建。
- 客户端按需拉取（合并请求、不阻塞、未就绪单元按空处理）。
- **协议不兼容**：`WIRE_VERSION` `14 → 15`，版本号 **`y+1` → `0.15.0`**，须附迁移指南 + 契约 YAML 同步 + BarekHistory 条目。

### 决策 3 · compute 主导管线 + 极薄放大 pass

- **Compute pass** 按"像素块列表"逐块算区域光照，写入低分辨率存储纹理（每块 5 条光线上限，结果块内共享）。
- **放大 pass** 以 nearest 采样放大到 1080p / 2K，**不含任何光照逻辑**。
- WGSL 按 `common / grid / trace / shade / raster / upscale` 多文件 `#import` 组合，**一模块一 entry point**，杜绝单片着色器。

### 决策 4 · 客户端新增顶层模块 `HostCode/shader/`

- 模块边界即文件边界：`grid`（M1）/ `region`（M2）/ `trace`（M3+M4）/ `shade`（M5）/ `raster`（M6）/ `backend`（M7）/ `debug`。
- 依赖方向：`shader → flow（资源）→ contract`；`shader` **不直连 `net`**、**不 `use` `ServerCode`**。
- 单元请求经 `flow` 资源 + 事件，`net` 适配器消费（复刻 `WorldCatalog` 范式）。

## 后果（Consequences）

**正面**

- 后端覆盖由 wgpu 免费提供，省掉数万行手写后端代码，落在"几千行"预算内。
- 算法模块与后端解耦，可独立单测（CPU 参考实现）与独立 mock。
- 与"服务端权威 + 客户端只画"的既有分层一致：加速结构由服务端构建，客户端只消费。

**负面 / 代价**

- **放弃 DX11**：极老设备（仅 DX11）无法运行该套件 —— 已在文档中显式声明，非缺陷。
- **新增协议不兼容变更**（`y+1`）：双端必须同时升级，无渐进兼容路径。
- **按需拉取引入往返延迟**：首帧单元逐步填充、画面"渐进完整"；未就绪单元按空处理不阻塞。
- 触碰 `net`（L2），须同 PR 补 `ServerCode/net/module.md`，且属单人维护期 L2 变更（豁免 ≥2 reviewer，须满足四条并声明）。

## 备选方案（Alternatives）

| 方案 | 否决理由 |
|---|---|
| 手写 SPIR-V + 五后端（照抄范式） | 代码量与维护面远超预算，与仓库三条红线冲突，且 DX11 仍无法由 wgpu 之外的路径低成本覆盖 |
| 客户端本地构建 SceneGrid（不碰协议） | 与"服务端权威、单一事实来源"的精神不符；用户裁决选择服务端构建下发 |
| 全量一次性下发 / 按 AOI 分片 | 一次性下发首包带宽大；AOI 分片会致区域阴影/间接光跨片丢失命中，复杂度暴涨 |
| 纯 fragment pass（逐像素共享块结果） | 需跨片元同步块结果，极易退化成"逐像素求交"——正是本套件要避免的 |

## 关联

- [计划 0001 · 区域光照着色器套件](../plans/0001-区域光照着色器套件.md)
- [模块边界总览](../architecture/module-boundaries.md)
- [ADR 0003](0003-contract-crate.md) / [ADR 0004](0004-client-layer-convergence.md) / [ADR 0006](0006-small-fixed-packet-dual-channel.md)
- [BarekHistory](../barek-history.md)（实现时追加 `0.15.0` 条目）