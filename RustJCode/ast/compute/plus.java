/*
 * 二元加法节点（左 + 右）。
 *
 * 做什么：表示 `a + b`。
 * 提供什么功能：emit 按栈机求值——左值入栈、右值算入 EAX、弹出左值到 ECX，
 * 最后 `add eax, ecx`。加法满足交换律。
 */
package ast.compute;

import ast.expr;
import backend.x64;

public final class plus extends expr {
    private final expr left;
    private final expr right;

    public plus(expr left, expr right) {
        this.left = left;
        this.right = right;
    }

    @Override
    public void emit(x64 out) {
        left.emit(out);
        out.pushEax();
        right.emit(out);
        out.popEcx();
        out.addEaxEcx();
    }
}