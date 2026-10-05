/*
 * x86-64 指令字节缓冲与标号回填。
 *
 * 做什么：为 x64 后端提供可随机写回的字节缓冲，并承载相对跳转的标号分配与回填，
 *   使 x64 类本身只专注每条指令的编码。
 * 提供什么功能：
 *   - put / putInt：追加原始字节与 32 位小端整数；
 *   - newLabel / bind：分配标号并把它绑定到当前位置；
 *   - jump / branchIfZero：发出 jmp / jz（test eax,eax 后的条件跳转）并登记待回填位置；
 *   - bytes：回填全部相对位移后返回 .text 字节流。
 */
package backend;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public final class codebuffer {
    private byte[] buf = new byte[256];
    private int len;
    private final List<Integer> labelPos = new ArrayList<>();
    private final List<int[]> patches = new ArrayList<>();

    void put(int... bytes) {
        for (int b : bytes) write(b);
    }

    void putInt(int value) {
        write(value & 0xFF);
        write((value >>> 8) & 0xFF);
        write((value >>> 16) & 0xFF);
        write((value >>> 24) & 0xFF);
    }

    int newLabel() {
        labelPos.add(-1);
        return labelPos.size() - 1;
    }

    void bind(int label) {
        labelPos.set(label, len);
    }

    void jump(int label) {
        put(0xE9);          // jmp rel32
        patch(label);
    }

    void branchIfZero(int label) {
        put(0x85, 0xC0);    // test eax, eax
        put(0x0F, 0x84);    // jz rel32
        patch(label);
    }

    byte[] bytes() {
        for (int[] p : patches) {
            writeIntAt(p[1], labelPos.get(p[0]) - (p[1] + 4));
        }
        return Arrays.copyOf(buf, len);
    }

    /* 占用 4 字节相对位移并登记回填点（下一指令偏移 = 该点 + 4）。 */
    private void patch(int label) {
        int site = len;
        putInt(0);
        patches.add(new int[] { label, site });
    }

    private void write(int b) {
        if (len == buf.length) grow();
        buf[len++] = (byte) b;
    }

    private void writeIntAt(int pos, int value) {
        buf[pos] = (byte) (value & 0xFF);
        buf[pos + 1] = (byte) ((value >>> 8) & 0xFF);
        buf[pos + 2] = (byte) ((value >>> 16) & 0xFF);
        buf[pos + 3] = (byte) ((value >>> 24) & 0xFF);
    }

    private void grow() {
        byte[] bigger = new byte[buf.length * 2];
        System.arraycopy(buf, 0, bigger, 0, len);
        buf = bigger;
    }
}