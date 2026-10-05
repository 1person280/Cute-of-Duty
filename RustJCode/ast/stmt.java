/*
 * 语句 AST 抽象基类（纯数据）。
 *
 * 做什么：作为所有语句节点的公共父类，仅用于类型归组。
 * 提供什么功能：无行为、无字段；具体数据由 letstmt 等子类承载。
 * 发射由 backend.codegen 分派。
 */
package ast;

public abstract class stmt {
}