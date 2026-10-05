/*
 * 整数字面量表达式节点。
 *
 * 做什么：承载源码中的 i32 字面量，例如 `1`、`42`。
 *
 * 提供什么功能：
 *   - intlit(int value)：保存字面量数值。
 *   - emit(x64 out)：发射 `mov eax, imm32`，把常量装入 EAX。
 */
package ast;

import backend.x64;

public final class intlit extends expr {
    private final int value;

    public intlit(int value) {
        this.value = value;
    }

    @Override
    public void emit(x64 out) {
        out.movEaxImm(value);
    }
}