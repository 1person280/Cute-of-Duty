/*
 * x86-64 机器码发射器（backend.arch 的 win-x64 实现）。
 *
 * 做什么：把抽象栈机指令编码为 x86-64 原始指令字节，并负责 rbp 栈帧的建立与拆除；
 *   字节写入与标号回填委托给 codebuffer。
 * 提供什么功能：实现 backend.arch 全部方法——序言/尾声、常量与变量槽读写、栈机搬运、
 *   算术/比较、标号与相对跳转，最后 finish() 返回 .text 段字节流。
 * 约定：求值结果统一落在 EAX，它同时是函数返回值与进程退出码。
 */
package backend;

public final class x64 implements arch {
    private final codebuffer buf = new codebuffer();

    /* 序言：push rbp ; mov rbp, rsp ; sub rsp, 16 字节对齐后的栈帧大小。 */
    public void begin(int frameBytes) {
        buf.put(0x55);
        buf.put(0x48, 0x89, 0xE5);
        int frame = (frameBytes + 15) & ~15;
        if (frame > 0) {
            buf.put(0x48, 0x81, 0xEC);
            buf.putInt(frame);
        }
    }

    /* 尾声：mov rsp, rbp ; pop rbp ; ret（EAX 即返回值/退出码）。 */
    public void end() {
        buf.put(0x48, 0x89, 0xEC);
        buf.put(0x5D);
        buf.put(0xC3);
    }

    public void movEaxImm(int value) { buf.put(0xB8); buf.putInt(value); }
    public void loadEax(int disp) { buf.put(0x8B, 0x85); buf.putInt(disp); }
    public void storeEax(int disp) { buf.put(0x89, 0x85); buf.putInt(disp); }

    public void pushEax() { buf.put(0x50); }
    public void popEcx() { buf.put(0x59); }
    public void popEax() { buf.put(0x58); }
    public void movEcxEax() { buf.put(0x89, 0xC1); }
    public void movEaxEdx() { buf.put(0x89, 0xD0); }

    public void addEaxEcx() { buf.put(0x03, 0xC1); }
    public void subEaxEcx() { buf.put(0x2B, 0xC1); }
    public void imulEaxEcx() { buf.put(0x0F, 0xAF, 0xC1); }
    public void negEax() { buf.put(0xF7, 0xD8); }
    public void cdq() { buf.put(0x99); }
    public void idivEcx() { buf.put(0xF7, 0xF9); }

    public void cmpEaxEcx() { buf.put(0x3B, 0xC1); }
    public void setE() { buf.put(0x0F, 0x94, 0xC0); }
    public void setNE() { buf.put(0x0F, 0x95, 0xC0); }
    public void setL() { buf.put(0x0F, 0x9C, 0xC0); }
    public void setLE() { buf.put(0x0F, 0x9E, 0xC0); }
    public void setG() { buf.put(0x0F, 0x9F, 0xC0); }
    public void setGE() { buf.put(0x0F, 0x9D, 0xC0); }
    public void movzxEaxAl() { buf.put(0x0F, 0xB6, 0xC0); }

    public int newLabel() { return buf.newLabel(); }
    public void bind(int label) { buf.bind(label); }
    public void jump(int label) { buf.jump(label); }
    public void branchIfZero(int label) { buf.branchIfZero(label); }

    public byte[] finish() {
        return buf.bytes();
    }
}