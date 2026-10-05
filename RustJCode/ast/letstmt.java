/*
 * let 绑定语句（纯数据）。
 *
 * 做什么：表示函数/块内的一条 `let 名字 = 表达式;`。
 * 提供什么功能：字段 offset 为该变量的栈帧偏移，init 为初始化表达式。
 * 变量名不在此保存——语法分析期已由 frontend.locals 转成 offset。
 */
package ast;

public final class letstmt extends stmt {
    public final int offset;
    public final expr init;

    public letstmt(int offset, expr init) {
        this.offset = offset;
        this.init = init;
    }
}