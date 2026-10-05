/*
 * if / else 语句（纯数据）。
 *
 * 做什么：表示 `if 条件 <块> else <块>`，else 块可缺省。
 * 提供什么功能：字段 cond 为条件表达式（i32，0 视为假、非 0 为真），
 *   then 为真分支块，els 为假分支块（无 else 时为 null）。
 */
package ast;

public final class ifstmt extends stmt {
    public final expr cond;
    public final block then;
    public final block els;

    public ifstmt(expr cond, block then, block els) {
        this.cond = cond;
        this.then = then;
        this.els = els;
    }
}