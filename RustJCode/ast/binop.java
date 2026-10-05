/*
 * 二元运算表达式节点（纯数据）。
 *
 * 做什么：表示 `左 <op> 右`，op 由 ast.op 常量指明（算术或比较）。
 * 提供什么功能：字段 kind 指明运算种类，left / right 为操作数子表达式。
 * 发射由 backend.eval 按 kind 分派，本节点不含任何机器码逻辑。
 */
package ast;

public final class binop extends expr {
    public final int kind;
    public final expr left;
    public final expr right;

    public binop(int kind, expr left, expr right) {
        this.kind = kind;
        this.left = left;
        this.right = right;
    }
}