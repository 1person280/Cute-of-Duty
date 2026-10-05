/*
 * 二元减法节点（左 - 右）。
 *
 * 做什么：表示 `a - b`。
 * 提供什么功能：emit 按栈机求值并保序——左值入栈、右值算入 EAX 后转入 ECX、
 * 弹出左值回 EAX，最后 `sub eax, ecx`。减法不满足交换律。
 */
package ast.compute;

import ast.expr;
import backend.x64;

public final class minus extends expr {
    private final expr left;
    private final expr right;

    public minus(expr left, expr right) {
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
        out.subEaxEcx();     // eax = 左 - 右
    }
}