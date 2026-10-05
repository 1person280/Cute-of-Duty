/*
 * 表达式 AST 抽象基类（纯数据）。
 *
 * 做什么：作为所有表达式节点的公共父类，仅用于类型归组，不再携带发射行为。
 *
 * 提供什么功能：无行为、无字段，只提供类型身份；具体数据由子类
 *   intlit / ident / binop / unop 承载。
 *
 * 为什么不含 emit：机器码发射属硬件架构相关逻辑，已移出 AST 交给 backend；
 * 这样 AST 与目标架构解耦，新增架构（如 arm64）无需改动任何节点。
 */
package ast;

public abstract class expr {
}