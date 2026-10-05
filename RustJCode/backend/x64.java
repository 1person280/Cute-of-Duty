/*
 * x86-64 机器码发射器（backend.arch 的 win-x64 实现）。
 *
 * 做什么：把抽象栈机指令编码为 x86-64 原始指令字节，并负责 rbp 栈帧的建立与拆除。
 * 提供什么功能：实现 backend.arch 全部方法——序言/尾声、常量与变量槽读写、栈机搬运、
 *   算术与比较指令，最后 finish() 返回 .text 段字节流。
 * 约定：求值结果统一落在 EAX，它同时是函数返回值与进程退出码。
 */
package backend;

import java.io.ByteArrayOutputStream;

public final class x64 implements arch {
    private final ByteArrayOutputStream code = new ByteArrayOutputStream();

    /* 序言：push rbp ; mov rbp, rsp ; sub rsp, 16 字节对齐后的栈帧大小。 */
    public void begin(int frameBytes) {
        put(0x55);
        put(0x48, 0x89, 0xE5);
        int frame = (frameBytes + 15) & ~15;
        if (frame > 0) {
            put(0x48, 0x81, 0xEC);
            putInt(frame);
        }
    }

    /* 尾声：mov rsp, rbp ; pop rbp ; ret（EAX 即返回值/退出码）。 */
    public void end() {
        put(0x48, 0x89, 0xEC);
        put(0x5D);
        put(0xC3);
    }

    public void movEaxImm(int value) { put(0xB8); putInt(value); }
    public void loadEax(int disp) { put(0x8B, 0x85); putInt(disp); }
    public void storeEax(int disp) { put(0x89, 0x85); putInt(disp); }

    public void pushEax() { put(0x50); }
    public void popEcx() { put(0x59); }
    public void popEax() { put(0x58); }
    public void movEcxEax() { put(0x89, 0xC1); }
    public void movEaxEdx() { put(0x89, 0xD0); }

    public void addEaxEcx() { put(0x03, 0xC1); }
    public void subEaxEcx() { put(0x2B, 0xC1); }
    public void imulEaxEcx() { put(0x0F, 0xAF, 0xC1); }
    public void negEax() { put(0xF7, 0xD8); }
    public void cdq() { put(0x99); }
    public void idivEcx() { put(0xF7, 0xF9); }

    public void cmpEaxEcx() { put(0x3B, 0xC1); }
    public void setE() { put(0x0F, 0x94, 0xC0); }
    public void setNE() { put(0x0F, 0x95, 0xC0); }
    public void setL() { put(0x0F, 0x9C, 0xC0); }
    public void setLE() { put(0x0F, 0x9E, 0xC0); }
    public void setG() { put(0x0F, 0x9F, 0xC0); }
    public void setGE() { put(0x0F, 0x9D, 0xC0); }
    public void movzxEaxAl() { put(0x0F, 0xB6, 0xC0); }

    public byte[] finish() {
        return code.toByteArray();
    }

    private void put(int... bytes) {
        for (int b : bytes) code.write(b);
    }

    private void putInt(int value) {
        code.write(value & 0xFF);
        code.write((value >>> 8) & 0xFF);
        code.write((value >>> 16) & 0xFF);
        code.write((value >>> 24) & 0xFF);
    }
}