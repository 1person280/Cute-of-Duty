/*
 * 一元运算表达式节点（纯数据）。
 *
 * 做什么：表示 `-x` 形式的单目运算。
 * 提供什么功能：字段 kind 指明运算种类（当前仅 op.NEG），operand 为操作数。
 * 发射由 backend.eval 按 kind 分派，本节点不含任何机器码逻辑。
 */
package ast;

public final class unop extends expr {
    public final int kind;
    public final expr operand;

    public unop(int kind, expr operand) {
        this.kind = kind;
        this.operand = operand;
    }
}