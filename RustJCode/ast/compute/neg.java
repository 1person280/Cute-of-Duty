/*
 * 一元取负节点（-x）。
 *
 * 做什么：表示对数取负，如 `-a`、`-(a + b)`。
 * 提供什么功能：emit 先对子表达式求值到 EAX，再 `neg eax` 求补。
 * 优先级：高于 `* / %`，因此 `-a * b` 解析为 `(-a) * b`。
 */
package ast.compute;

import ast.expr;
import backend.x64;

public final class neg extends expr {
    private final expr operand;

    public neg(expr operand) {
        this.operand = operand;
    }

    @Override
    public void emit(x64 out) {
        operand.emit(out);
        out.negEax();
    }
}