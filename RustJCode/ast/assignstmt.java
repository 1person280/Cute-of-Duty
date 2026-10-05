/*
 * 赋值语句（纯数据）。
 *
 * 做什么：表示 `名字 = 表达式;`，把右值写回一个已声明变量的栈槽。
 * 提供什么功能：字段 offset 为目标变量偏移，value 为右值表达式。
 * 变量名已在语法分析期由 frontend.locals 解析为 offset。
 */
package ast;

public final class assignstmt extends stmt {
    public final int offset;
    public final expr value;

    public assignstmt(int offset, expr value) {
        this.offset = offset;
        this.value = value;
    }
}