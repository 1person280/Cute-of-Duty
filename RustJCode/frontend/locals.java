/*
 * 局部变量符号表。
 *
 * 做什么：为函数体内的 let 变量分配栈帧偏移，并在变量被引用时校验其声明合法性。
 *
 * 提供什么功能：
 *   - declare(String name, int line)：登记新变量，返回其 rbp 相对偏移（-4, -8, ...）；
 *     重复声明时报错。
 *   - offsetOf(String name, int line)：按名查偏移；未声明时报错。
 *   - size()：已声明变量总数，供语法分析期算出栈帧字节数。
 *   - 使语法分析期即可同步完成「名字 → 偏移」解析与最基本的语义检查。
 */
package frontend;

import error.rustjerror;
import java.util.HashMap;
import java.util.Map;

public final class locals {
    private final Map<String, Integer> offsets = new HashMap<>();
    private int count;

    public int declare(String name, int line) {
        if (offsets.containsKey(name)) {
            throw new rustjerror(line, "变量重复声明: " + name);
        }
        count++;
        int offset = -4 * count;
        offsets.put(name, offset);
        return offset;
    }

    public int offsetOf(String name, int line) {
        Integer offset = offsets.get(name);
        if (offset == null) {
            throw new rustjerror(line, "未声明的变量: " + name);
        }
        return offset;
    }

    public int size() {
        return count;
    }
}