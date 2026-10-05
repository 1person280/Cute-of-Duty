/*
 * let 绑定语句。
 *
 * 做什么：表示函数体内的一条 `let 名字 = 表达式;`，是本子集唯一的语句形态。
 *
 * 提供什么功能：
 *   - letstmt(int offset, expr init)：持有该变量的栈帧偏移与其初始化表达式。
 *   - 字段 offset 与 init 供代码生成阶段读取：先对 init 求值，再把结果存入 offset 槽。
 *
 * 变量名不在此保存——它在语法分析期已由 frontend.locals 转成 offset。
 */
package ast;

public final class letstmt {
    public final int offset;
    public final expr init;

    public letstmt(int offset, expr init) {
        this.offset = offset;
        this.init = init;
    }
}