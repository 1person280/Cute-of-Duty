/*
 * 小于比较节点（左 < 右），有符号。
 *
 * 做什么：判断左是否小于右。
 * 提供什么功能：emit 以 `cmp` 置标志位，`setl al` 取标志（SF ≠ OF），`movzx` 零扩展；
 * 结果为 i32 的 1（真）或 0（假）。
 */
package ast.compute;

import ast.expr;
import backend.x64;

public final class lt extends expr {
    private final expr left;
    private final expr right;

    public lt(expr left, expr right) {
        this.left = left;
        this.right = right;
    }

    @Override
    public void emit(x64 out) {
        left.emit(out);
        out.pushEax();
        right.emit(out);
        out.movEcxEax();
        out.popEax();
        out.cmpEaxEcx();
        out.setL();
        out.movzxEaxAl();
    }
}