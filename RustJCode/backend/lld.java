/*
 * 自研链接器（纯 Java，替代 rust-lld）。
 *
 * 做什么：读取最小 PE/COFF 目标文件，定位入口符号与 .text 段，
 *         再把段内容交给 pe 封装为可执行文件——全程不依赖任何外部二进制。
 *
 * 提供什么功能：
 *   - link(byte[] object)：校验入口符号存在、取出 .text 段，产出 win-x64 PE exe。
 *
 * 当前仅支持最小 COFF 子集：单 .text 段、无重定位、无外部导入。
 */
package backend;

import error.rustjerror;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;

public final class lld {
    private lld() {
    }

    public static byte[] link(byte[] object) {
        ByteBuffer b = ByteBuffer.wrap(object).order(ByteOrder.LITTLE_ENDIAN);
        int sections = b.getShort(2) & 0xFFFF;
        int symPtr = b.getInt(8);
        int symbols = b.getInt(12);
        int strtab = symPtr + symbols * 18;
        if (!hasSymbol(b, symPtr, symbols, strtab, "main")) {
            throw new rustjerror(0, "链接器：目标文件缺少外部符号 main");
        }
        return pe.write(readText(b, sections));
    }

    private static boolean hasSymbol(ByteBuffer b, int symPtr, int symbols, int strtab, String want) {
        for (int i = 0; i < symbols; i++) {
            if (name(b, symPtr + i * 18, strtab).equals(want)) return true;
        }
        return false;
    }

    private static byte[] readText(ByteBuffer b, int sections) {
        for (int i = 0; i < sections; i++) {
            int off = 20 + i * 40;
            if (!rawName(b, off).equals(".text")) continue;
            int size = b.getInt(off + 16);        // SizeOfRawData
            int ptr = b.getInt(off + 20);         // PointerToRawData
            byte[] data = new byte[size];
            for (int j = 0; j < size; j++) data[j] = b.get(ptr + j);
            return data;
        }
        throw new rustjerror(0, "链接器：目标文件缺少 .text 段");
    }

    private static String name(ByteBuffer b, int off, int strtab) {
        if (b.getInt(off) != 0) return rawName(b, off);
        return cstring(b, strtab + b.getInt(off + 4));
    }

    private static String rawName(ByteBuffer b, int off) {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 8; i++) {
            char c = (char) (b.get(off + i) & 0xFF);
            if (c == '\0') break;
            sb.append(c);
        }
        return sb.toString();
    }

    private static String cstring(ByteBuffer b, int off) {
        StringBuilder sb = new StringBuilder();
        for (int i = off; i < b.capacity(); i++) {
            char c = (char) (b.get(i) & 0xFF);
            if (c == '\0') break;
            sb.append(c);
        }
        return sb.toString();
    }
}