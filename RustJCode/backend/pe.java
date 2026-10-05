/*
 * win-x64 PE 可执行文件写出器。
 *
 * 做什么：把一段已定位的机器码封装为可直接运行的 PE32+ 可执行文件，
 *         手写 DOS/NT 头、可选头、节表，并保持导入表为空（零 OS import）。
 *
 * 提供什么功能：
 *   - write(byte[] text)：产出 PE exe；入口点即 .text 起始，
 *     进程退出码由入口函数返回的 EAX 决定。
 *
 * 当前仅支持单节 .text、无导入、无重定位的最小可执行镜像。
 */
package backend;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;

public final class pe {
    private static final int FILE_ALIGN = 0x200;
    private static final int SECTION_ALIGN = 0x1000;
    private static final long IMAGE_BASE = 0x140000000L;

    private pe() {
    }

    public static byte[] write(byte[] text) {
        int rawSize = align(text.length, FILE_ALIGN);
        int imageSize = align(SECTION_ALIGN + text.length, SECTION_ALIGN);
        ByteBuffer b = ByteBuffer.allocate(FILE_ALIGN + rawSize).order(ByteOrder.LITTLE_ENDIAN);
        // DOS 头
        b.putShort(0x00, (short) 0x5A4D);         // "MZ"
        b.putInt(0x3C, 0x80);                     // e_lfanew -> PE 头
        // PE 签名
        b.putInt(0x80, 0x00004550);               // "PE\0\0"
        // IMAGE_FILE_HEADER（0x84）
        b.putShort(0x84, (short) 0x8664);         // Machine = AMD64
        b.putShort(0x86, (short) 1);              // NumberOfSections
        b.putShort(0x94, (short) 240);            // SizeOfOptionalHeader
        b.putShort(0x96, (short) 0x0022);         // EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE
        // IMAGE_OPTIONAL_HEADER64（0x98）
        b.putShort(0x98, (short) 0x20B);          // PE32+
        b.putInt(0x9C, rawSize);                  // SizeOfCode
        b.putInt(0xA8, SECTION_ALIGN);            // AddressOfEntryPoint
        b.putInt(0xAC, SECTION_ALIGN);            // BaseOfCode
        b.putLong(0xB0, IMAGE_BASE);              // ImageBase
        b.putInt(0xB8, SECTION_ALIGN);            // SectionAlignment
        b.putInt(0xBC, FILE_ALIGN);               // FileAlignment
        b.putShort(0xC0, (short) 6);              // MajorOperatingSystemVersion
        b.putShort(0xC8, (short) 6);              // MajorSubsystemVersion
        b.putInt(0xD0, imageSize);                // SizeOfImage
        b.putInt(0xD4, FILE_ALIGN);               // SizeOfHeaders
        b.putShort(0xDC, (short) 3);              // Subsystem = Console
        b.putLong(0xE0, 0x100000L);               // SizeOfStackReserve
        b.putLong(0xE8, 0x1000L);                 // SizeOfStackCommit
        b.putLong(0xF0, 0x100000L);               // SizeOfHeapReserve
        b.putLong(0xF8, 0x1000L);                 // SizeOfHeapCommit
        b.putInt(0x104, 16);                      // NumberOfRvaAndSizes（数据目录全零=无导入）
        // IMAGE_SECTION_HEADER（0x188）
        byte[] name = ".text".getBytes(StandardCharsets.US_ASCII);
        for (int i = 0; i < name.length; i++) b.put(0x188 + i, name[i]);
        b.putInt(0x190, text.length);             // VirtualSize
        b.putInt(0x194, SECTION_ALIGN);           // VirtualAddress
        b.putInt(0x198, rawSize);                 // SizeOfRawData
        b.putInt(0x19C, FILE_ALIGN);              // PointerToRawData
        b.putInt(0x1AC, 0x60000020);              // CODE | EXECUTE | READ
        // 代码
        b.position(FILE_ALIGN);
        b.put(text);
        return b.array();
    }

    private static int align(int value, int alignment) {
        return (value + alignment - 1) / alignment * alignment;
    }
}