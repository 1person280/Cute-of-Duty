/*
 * 变量引用表达式节点。
 *
 * 做什么：表示对某个已声明局部变量（let）的读取，例如 `x`。
 *
 * 提供什么功能：
 *   - ident(int offset)：保存由符号表解析出的 rbp 相对偏移。
 *   - emit(x64 out)：发射 `mov eax, [rbp+disp32]`，把变量值装入 EAX。
 *
 * 偏移在语法分析期由 frontend.locals 查表确定，因此本节点不再持有变量名，
 * 也无需在代码生成阶段回查符号表。
 */
package ast;

import backend.x64;

public final class ident extends expr {
    private final int offset;

    public ident(int offset) {
        this.offset = offset;
    }

    @Override
    public void emit(x64 out) {
        out.loadEax(offset);
    }
}