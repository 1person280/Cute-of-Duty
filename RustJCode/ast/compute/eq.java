/*
 * 相等比较节点（左 == 右）。
 *
 * 做什么：比较两值是否相等。
 * 提供什么功能：emit 以 `cmp` 置标志位，`sete al` 取标志，`movzx` 零扩展；
 * 结果为 i32 的 1（真）或 0（假）——本子集暂不引入独立 bool 类型。
 */
package ast.compute;

import ast.expr;
import backend.x64;

public final class eq extends expr {
    private final expr left;
    private final expr right;

    public eq(expr left, expr right) {
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
        out.setE();
        out.movzxEaxAl();
    }
}