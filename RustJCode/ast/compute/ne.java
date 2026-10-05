/*
 * 不等比较节点（左 != 右）。
 *
 * 做什么：比较两值是否不相等。
 * 提供什么功能：emit 以 `cmp` 置标志位，`setne al` 取标志，`movzx` 零扩展；
 * 结果为 i32 的 1（真）或 0（假）。
 */
package ast.compute;

import ast.expr;
import backend.x64;

public final class ne extends expr {
    private final expr left;
    private final expr right;

    public ne(expr left, expr right) {
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
        out.setNE();
        out.movzxEaxAl();
    }
}