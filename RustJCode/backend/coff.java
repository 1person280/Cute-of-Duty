/*
 * PE/COFF 目标文件写出器。
 *
 * 做什么：把一段已编码的机器码（.text）包装为标准 COFF 目标文件（win-x64），
 *         采用标准格式是为了给未来接入真实 .o / rlib 保留通路。
 *
 * 提供什么功能：
 *   - write(String symbol, byte[] text)：产出含单个 .text 段与单个外部符号的
 *     最小 COFF 目标文件；符号名经字符串表保存。
 *
 * 当前仅支持最小子集：NumberOfSections=1、无重定位、无可选头。
 */
package backend;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;

public final class coff {
    private static final int MACHINE_AMD64 = 0x8664;
    private static final int SECTION_CHARS = 0x60000020;   // CODE | EXECUTE | READ
    private static final int SYMBOL_EXTERNAL = 2;

    private coff() {
    }

    public static byte[] write(String symbol, byte[] text) {
        byte[] strtab = stringTable(symbol);
        int rawPtr = 20 + 40;
        int symPtr = rawPtr + text.length;
        ByteBuffer b = ByteBuffer.allocate(symPtr + 18 + strtab.length).order(ByteOrder.LITTLE_ENDIAN);
        // IMAGE_FILE_HEADER
        b.putShort((short) MACHINE_AMD64);
        b.putShort((short) 1);            // NumberOfSections
        b.putInt(0);                      // TimeDateStamp
        b.putInt(symPtr);                 // PointerToSymbolTable
        b.putInt(1);                      // NumberOfSymbols
        b.putShort((short) 0);            // SizeOfOptionalHeader
        b.putShort((short) 0);            // Characteristics
        // IMAGE_SECTION_HEADER (.text)
        b.put(sectionName(".text"));
        b.putInt(text.length);            // VirtualSize
        b.putInt(0);                      // VirtualAddress
        b.putInt(text.length);            // SizeOfRawData
        b.putInt(rawPtr);                 // PointerToRawData
        b.putInt(0);                      // PointerToRelocations
        b.putInt(0);                      // PointerToLinenumbers
        b.putShort((short) 0);            // NumberOfRelocations
        b.putShort((short) 0);            // NumberOfLinenumbers
        b.putInt(SECTION_CHARS);          // Characteristics
        // 段数据
        b.put(text);
        // 符号：Name = 0 + 字符串表偏移(4)
        b.putInt(0);
        b.putInt(4);
        b.putInt(0);                      // Value
        b.putShort((short) 1);            // SectionNumber
        b.putShort((short) 0);            // Type
        b.put((byte) SYMBOL_EXTERNAL);    // StorageClass
        b.put((byte) 0);                  // NumberOfAuxSymbols
        // 字符串表
        b.put(strtab);
        return b.array();
    }

    private static byte[] sectionName(String name) {
        byte[] out = new byte[8];
        byte[] raw = name.getBytes(StandardCharsets.US_ASCII);
        System.arraycopy(raw, 0, out, 0, Math.min(raw.length, out.length));
        return out;
    }

    private static byte[] stringTable(String name) {
        byte[] raw = name.getBytes(StandardCharsets.US_ASCII);
        ByteBuffer b = ByteBuffer.allocate(4 + raw.length + 1).order(ByteOrder.LITTLE_ENDIAN);
        b.putInt(4 + raw.length + 1);     // 长度字段含自身
        b.put(raw);
        b.put((byte) 0);
        return b.array();
    }
}