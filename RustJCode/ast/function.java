/*
 * 一个函数的语法树表示。
 *
 * 做什么：承载 `fn 名字() -> i32 { <let 序列> <返回表达式> }` 的整体结构。
 *
 * 提供什么功能：
 *   - function(String name, List<letstmt> lets, expr body)：保存函数名、
 *     按源码顺序排列的 let 绑定，以及作为返回值的最终表达式。
 *   - 字段 name 决定产物符号名与输出文件名；
 *     字段 lets 决定栈帧大小；字段 body 提供函数的返回值（EAX）。
 */
package ast;

import java.util.List;

public final class function {
    public final String name;
    public final List<letstmt> lets;
    public final expr body;

    public function(String name, List<letstmt> lets, expr body) {
        this.name = name;
        this.lets = lets;
        this.body = body;
    }
}