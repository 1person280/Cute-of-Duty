/*
 * 大于比较节点（左 > 右），有符号。
 *
 * 做什么：判断左是否大于右。
 * 提供什么功能：emit 以 `cmp` 置标志位，`setg al` 取标志（ZF=0 且 SF=OF），
 * `movzx` 零扩展；结果为 i32 的 1（真）或 0（假）。
 */
package ast.compute;

import ast.expr;
import backend.x64;

public final class gt extends expr {
    private final expr left;
    private final expr right;

    public gt(expr left, expr right) {
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
        out.setG();
        out.movzxEaxAl();
    }
}