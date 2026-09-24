//! AOI 兴趣区域剔除（README §4.3）
//!
//! 每个客户端只收到**其视野范围内 / AOI 兴趣区域内**的实体数据；
//! 超出范围的实体不参与该客户端的同步。既降低 TCP 带宽压力，
//! 也从根源杜绝 ESP 透视外挂——客户端根本不知道视野外有什么。

/// 默认 AOI 兴趣半径（单位：米）。
///
/// 结合长 TTK 的设计（基础击杀 ~5s）不要求全图同步，60m 已涵盖常规交战距离；
/// 数值后续可收敛进配置表驱动（当前为服务端权威常量）。
pub const AOI_RADIUS: f32 = 60.0;

/// 判断目标坐标是否落在观察者兴趣范围内。
///
/// 使用平方距离避免开方，供高频率快照过滤时保持低开销、确定性一致。
pub fn in_interest(
    target: (f32, f32, f32),
    observer: (f32, f32, f32),
    radius: f32,
) -> bool {
    let dx = target.0 - observer.0;
    let dy = target.1 - observer.1;
    let dz = target.2 - observer.2;
    dx * dx + dy * dy + dz * dz <= radius * radius
}