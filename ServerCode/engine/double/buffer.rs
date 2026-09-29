//! 双缓冲渲染快照
//!
//! 逻辑层与渲染层各自读写独立的 `RenderSnapshot`，通过 `swap` 原子槽位轮换，
//! 保证渲染线程始终拿到完整一致的一帧，避免与逻辑层共享可变缓冲。

use crate::entity::RenderSnapshot;

/// 双缓冲渲染快照
pub struct DoubleBuffer {
    /// 逻辑层写入的缓冲区
    logic_buffer: RenderSnapshot,
    /// 渲染层读取的缓冲区
    render_buffer: RenderSnapshot,
}

impl DoubleBuffer {
    pub fn new() -> Self {
        Self {
            logic_buffer: RenderSnapshot::new(0),
            render_buffer: RenderSnapshot::new(0),
        }
    }

    /// 获取逻辑写入缓冲区
    pub fn get_logic_buffer(&mut self) -> &mut RenderSnapshot {
        &mut self.logic_buffer
    }

    /// 获取渲染读取缓冲区（只读）
    pub fn get_render_buffer(&self) -> &RenderSnapshot {
        &self.render_buffer
    }

    /// 交换缓冲区
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.logic_buffer, &mut self.render_buffer);
    }
}

impl Default for DoubleBuffer {
    fn default() -> Self {
        Self::new()
    }
}