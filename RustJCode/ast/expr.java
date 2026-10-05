/*
 * 表达式 AST 抽象基类。
 *
 * 做什么：为所有表达式节点定义统一的「发射求值机器码」入口，使后端发射器
 *         无需按节点类型写分支，靠多态分发即可处理任意表达式树。
 *
 * 提供什么功能：
 *   - emit(x64 out)：发射本表达式的求值指令，结果落在 EAX；
 *     由具体节点 intlit / ident / plus / minus / times 分别实现。
 */
package ast;

import backend.x64;

public abstract class expr {
    /* 发射本表达式的求值机器码，结果置于 EAX。 */
    public abstract void emit(x64 out);
}