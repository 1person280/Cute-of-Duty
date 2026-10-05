/*
 * while 循环语句（纯数据）。
 *
 * 做什么：表示 `while 条件 <块>`，条件为真时反复执行循环体。
 * 提供什么功能：字段 cond 为条件表达式（i32，0 视为假），body 为循环体块。
 */
package ast;

public final class whilestmt extends stmt {
    public final expr cond;
    public final block body;

    public whilestmt(expr cond, block body) {
        this.cond = cond;
        this.body = body;
    }
}