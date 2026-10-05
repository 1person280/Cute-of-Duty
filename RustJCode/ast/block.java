/*
 * 语句块节点（纯数据）。
 *
 * 做什么：表示一对花括号内的 `语句* 尾表达式?`，是函数体、分支与循环体的统一载体。
 * 提供什么功能：
 *   - stmts：按源码顺序排列的语句列表；
 *   - tail：块末尾的可选表达式，其求值结果即该块的值（无则为 null）。
 * 块可嵌套，控制流因此自然递归。
 */
package ast;

import java.util.List;

public final class block {
    public final List<stmt> stmts;
    public final expr tail;

    public block(List<stmt> stmts, expr tail) {
        this.stmts = stmts;
        this.tail = tail;
    }
}