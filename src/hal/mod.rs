//! 硬件抽象层 (HAL)
//!
//! 设计目标：
//! - 时钟统一：所有硬件事件必须打上基于同一单调时钟源的时间戳
//! - 中断消解：硬件中断在HAL内转换为「带时戳的缓冲数据」
//! - 差异封装：x86/ARM、不同GPU厂商、不同网卡芯片的差异对上层不可见
//! - 零分配：HAL内部使用预分配环形缓冲区，中断处理路径禁止堆分配

use std::sync::OnceLock;
use std::time::Instant;
use crossbeam_queue::ArrayQueue;

/// 单调时钟接口
///
/// 返回自首次调用以来的微秒数。
/// 基于std::time::Instant实现：严格单调递增，不受NTP回拨影响，
/// 且初始化无竞态（OnceLock保证时钟基准只被设置一次）。
pub fn monotonic_us() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_micros() as u64
}

/// 时钟域类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockDomain {
    CPU,
    Mouse,
    Keyboard,
    Network,
    GPU,
}

impl ClockDomain {
    /// 获取时钟域偏移（纳秒）
    /// 
    /// 简化实现，实际应根据硬件校准
    pub fn offset_ns(&self) -> i64 {
        match self {
            ClockDomain::CPU => 0,
            ClockDomain::Mouse => 100,      // 鼠标固件时钟偏移
            ClockDomain::Keyboard => 200,   // 键盘时钟偏移
            ClockDomain::Network => 500,    // 网卡时钟偏移
            ClockDomain::GPU => 1000,       // GPU时钟偏移
        }
    }
}

/// 将硬件原始时间戳转换为单调时钟域
pub fn translate_timestamp(hw_ts: u64, domain: ClockDomain) -> u64 {
    let offset_us = domain.offset_ns() / 1000;
    (hw_ts as i64 + offset_us) as u64
}

/// 输入事件
#[derive(Debug, Clone)]
pub struct InputEvent {
    /// 单调时钟时间戳（微秒）
    pub timestamp_us: u64,
    /// 设备类型
    pub device_type: InputDeviceType,
    /// 事件类型
    pub event_type: InputEventType,
    /// 原始数据
    pub raw_data: [u8; 32],
    /// 数据长度
    pub data_len: usize,
}

/// 输入设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputDeviceType {
    Mouse,
    Keyboard,
    Gamepad,
    Touch,
}

/// 输入事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEventType {
    ButtonPress,
    ButtonRelease,
    AxisMove,
    PositionMove,
}

/// 输入系统常量
pub const INPUT_RING_CAPACITY: usize = 4096;

/// 输入HAL
pub struct InputHal {
    /// 环形缓冲区
    ring_buffer: ArrayQueue<InputEvent>,
}

impl InputHal {
    pub fn new() -> Self {
        Self {
            ring_buffer: ArrayQueue::new(INPUT_RING_CAPACITY),
        }
    }

    /// HID中断处理
    /// 
    /// 由硬件中断回调调用，应立即返回，不执行复杂逻辑
    pub fn on_hid_interrupt(&self, raw: &[u8], hw_ts: u64, device_type: InputDeviceType) {
        let ts = translate_timestamp(hw_ts, ClockDomain::Mouse);
        
        let mut event = InputEvent {
            timestamp_us: ts,
            device_type,
            event_type: InputEventType::ButtonPress, // 应由raw解析
            raw_data: [0; 32],
            data_len: raw.len().min(32),
        };
        
        event.raw_data[..event.data_len].copy_from_slice(&raw[..event.data_len]);
        
        // 尝试入队，溢出时丢弃最旧事件（在crossbeam中由ring_buffer自动处理）
        if self.ring_buffer.is_full() {
            // 丢弃最旧事件
            let _ = self.ring_buffer.pop();
        }
        
        let _ = self.ring_buffer.push(event);
    }

    /// 批量读取指定时间范围内的输入事件
    /// 
    /// 由逻辑Tick开始时调用
    pub fn flush_events(&self, tick_start_us: u64, tick_end_us: u64) -> Vec<InputEvent> {
        let mut events = Vec::new();
        
        // 收集时间范围内的所有事件
        while let Some(event) = self.ring_buffer.pop() {
            if event.timestamp_us >= tick_start_us && event.timestamp_us < tick_end_us {
                events.push(event);
            } else if event.timestamp_us < tick_start_us {
                // 过旧事件，丢弃
                continue;
            } else {
                // 未来事件，重新入队
                let _ = self.ring_buffer.push(event);
                break;
            }
        }
        
        // 按时间戳排序
        events.sort_by_key(|e| e.timestamp_us);
        events
    }

    /// 获取当前缓冲区事件数
    pub fn event_count(&self) -> usize {
        self.ring_buffer.len()
    }
}

impl Default for InputHal {
    fn default() -> Self {
        Self::new()
    }
}

/// 网络包描述符
#[derive(Debug, Clone)]
pub struct PacketDescriptor {
    pub timestamp_us: u64,
    pub data: Vec<u8>,
    pub source_addr: String,
}

/// 网络HAL常量
pub const RX_RING_SIZE: usize = 2048;
pub const TX_RING_SIZE: usize = 2048;

/// 网络HAL
pub struct NetworkHal {
    rx_ring: ArrayQueue<PacketDescriptor>,
    tx_ring: ArrayQueue<PacketDescriptor>,
}

impl NetworkHal {
    pub fn new() -> Self {
        Self {
            rx_ring: ArrayQueue::new(RX_RING_SIZE),
            tx_ring: ArrayQueue::new(TX_RING_SIZE),
        }
    }

    /// NIC中断处理
    pub fn on_nic_interrupt(&self, data: &[u8], hw_ts: u64, source_addr: &str) {
        let ts = translate_timestamp(hw_ts, ClockDomain::Network);
        
        let desc = PacketDescriptor {
            timestamp_us: ts,
            data: data.to_vec(),
            source_addr: source_addr.to_string(),
        };
        
        if self.rx_ring.is_full() {
            let _ = self.rx_ring.pop();
        }
        
        let _ = self.rx_ring.push(desc);
    }

    /// 批量读取入站数据包
    pub fn flush_inbound_packets(&self, tick_start_us: u64, tick_end_us: u64) -> Vec<PacketDescriptor> {
        let mut packets = Vec::new();
        
        while let Some(packet) = self.rx_ring.pop() {
            if packet.timestamp_us >= tick_start_us && packet.timestamp_us < tick_end_us {
                packets.push(packet);
            } else if packet.timestamp_us >= tick_end_us {
                let _ = self.rx_ring.push(packet);
                break;
            }
        }
        
        packets.sort_by_key(|p| p.timestamp_us);
        packets
    }

    /// 入站发送数据包
    pub fn enqueue_outbound(&self, packet: PacketDescriptor) {
        if self.tx_ring.is_full() {
            let _ = self.tx_ring.pop();
        }
        let _ = self.tx_ring.push(packet);
    }

    /// 批量获取出站数据包（由网络线程发送）
    pub fn flush_outbound_packets(&self) -> Vec<PacketDescriptor> {
        let mut packets = Vec::new();
        while let Some(packet) = self.tx_ring.pop() {
            packets.push(packet);
        }
        packets
    }
}

impl Default for NetworkHal {
    fn default() -> Self {
        Self::new()
    }
}

/// 显示HAL（简化版，实际应封装Vulkan/DX12）
pub struct DisplayHal {
    refresh_rate_hz: f32,
}

impl DisplayHal {
    pub fn new() -> Self {
        Self {
            refresh_rate_hz: 60.0,
        }
    }

    /// 查询当前刷新率
    pub fn current_refresh_rate_hz(&self) -> f32 {
        self.refresh_rate_hz
    }

    /// 设置刷新率
    pub fn set_refresh_rate(&mut self, hz: f32) {
        self.refresh_rate_hz = hz;
    }

    /// 提交GPU命令队列（简化版）
    pub fn submit_command_queue(&self, _commands: &[u8]) {
        // 实际实现应通过Vulkan/DX12提交命令
    }

    /// 等待VBlank
    pub fn wait_vblank(&self) {
        // 简化实现
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

impl Default for DisplayHal {
    fn default() -> Self {
        Self::new()
    }
}

/// HAL管理器
pub struct HalManager {
    pub input: InputHal,
    pub network: NetworkHal,
    pub display: DisplayHal,
}

impl HalManager {
    pub fn new() -> Self {
        Self {
            input: InputHal::new(),
            network: NetworkHal::new(),
            display: DisplayHal::new(),
        }
    }

    /// 初始化所有HAL
    pub fn initialize(&self) -> Result<(), HalError> {
        info!("HAL管理器初始化中...");
        info!("输入缓冲区容量: {}", INPUT_RING_CAPACITY);
        info!("网络RX缓冲区: {}, TX缓冲区: {}", RX_RING_SIZE, TX_RING_SIZE);
        info!("HAL管理器初始化完成");
        Ok(())
    }
}

impl Default for HalManager {
    fn default() -> Self {
        Self::new()
    }
}

/// HAL错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HalError {
    DeviceNotFound,
    InitializationFailed,
    BufferOverflow,
    UnsupportedFeature,
}

impl std::fmt::Display for HalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HalError::DeviceNotFound => write!(f, "硬件设备未找到"),
            HalError::InitializationFailed => write!(f, "硬件初始化失败"),
            HalError::BufferOverflow => write!(f, "缓冲区溢出"),
            HalError::UnsupportedFeature => write!(f, "不支持的硬件特性"),
        }
    }
}

impl std::error::Error for HalError {}

use tracing::info;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monotonic_clock() {
        let t1 = monotonic_us();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let t2 = monotonic_us();
        
        assert!(t2 > t1, "单调时钟必须递增");
    }

    #[test]
    fn test_input_hal() {
        let input = InputHal::new();
        
        // 模拟输入事件
        let raw = [0u8; 8];
        input.on_hid_interrupt(&raw, monotonic_us(), InputDeviceType::Mouse);
        
        assert_eq!(input.event_count(), 1);
        
        // 读取事件
        let now = monotonic_us();
        let events = input.flush_events(0, now + 1000);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_network_hal() {
        let network = NetworkHal::new();
        
        let packet = PacketDescriptor {
            timestamp_us: monotonic_us(),
            data: vec![1, 2, 3, 4],
            source_addr: "127.0.0.1:8080".to_string(),
        };
        
        network.enqueue_outbound(packet);
        
        let packets = network.flush_outbound_packets();
        assert_eq!(packets.len(), 1);
    }

    #[test]
    fn test_timestamp_translation() {
        let ts = 1000000u64;
        let translated = translate_timestamp(ts, ClockDomain::Mouse);
        
        // 鼠标域偏移100ns = 0.1μs
        assert_eq!(translated, ts + 0);
    }
}
