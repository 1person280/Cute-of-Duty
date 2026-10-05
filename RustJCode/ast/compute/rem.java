/*
 * 二元取余节点（左 % 右），有符号余数。
 *
 * 做什么：表示 `a % b`，结果的符号与被除数相同，与 Rust 的 `%` 一致。
 * 提供什么功能：emit 复用 `idiv` 的余数寄存器——求值后 `idiv ecx` 把余数留在 EDX，
 * 再 `mov eax, edx` 取回 EAX。
 * 注意：除数为 0 时由 CPU 触发 #DE 异常，本子集不额外拦截。
 */
package ast.compute;

import ast.expr;
import backend.x64;

public final class rem extends expr {
    private final expr left;
    private final expr right;

    public rem(expr left, expr right) {
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
        out.idivEcx();       // edx = 余数
        out.movEaxEdx();     // eax = 余数
    }
}