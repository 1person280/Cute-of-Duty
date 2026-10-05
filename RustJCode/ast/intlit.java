/*
 * 整数字面量表达式节点（纯数据）。
 *
 * 做什么：承载源码中的 i32 字面量，例如 `1`、`42`。
 * 提供什么功能：字段 value 保存字面量数值，供后端装入 EAX。
 */
package ast;

public final class intlit extends expr {
    public final int value;

    public intlit(int value) {
        this.value = value;
    }
}