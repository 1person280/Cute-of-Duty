/*
 * 二元除法节点（左 / 右），有符号整除。
 *
 * 做什么：表示 `a / b`，结果取商（向零截断），与 Rust 的 `/` 一致。
 * 提供什么功能：emit 在栈机求值后发出 `cdq`（把 EAX 符号扩展到 EDX:EAX）
 * 与 `idiv ecx`；商留在 EAX。
 * 注意：除数为 0 或商溢出时由 CPU 触发 #DE 异常，本子集不额外拦截。
 */
package ast.compute;

import ast.expr;
import backend.x64;

public final class div extends expr {
    private final expr left;
    private final expr right;

    public div(expr left, expr right) {
        this.left = left;
        this.right = right;
    }

    @Override
    public void emit(x64 out) {
        left.emit(out);      // eax = 左
        out.pushEax();
        right.emit(out);     // eax = 右
        out.movEcxEax();     // ecx = 右
        out.popEax();        // eax = 左
        out.cdq();           // 符号扩展到 edx:eax
        out.idivEcx();       // eax = 商
    }
}