/*
 * 变量引用表达式节点（纯数据）。
 *
 * 做什么：表示对某个已声明局部变量（let）的读取，例如 `x`。
 * 提供什么功能：字段 offset 保存由 frontend.locals 解析出的 rbp 相对偏移，
 *   供后端从栈帧读入 EAX；节点不持有变量名，代码生成期无需回查符号表。
 */
package ast;

public final class ident extends expr {
    public final int offset;

    public ident(int offset) {
        this.offset = offset;
    }
}