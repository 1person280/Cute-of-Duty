/*
 * 运算符种类常量。
 *
 * 做什么：用一个 int 编码区分 binop / unop 的具体运算，供前端构造、后端分派。
 * 提供什么功能：算术 `+ - * / %`、比较 `== != < <= > >=`、一元负号 NEG 的常量。
 *
 * 为什么用 int 而非每个运算一个节点类：节点类会随运算数量线性膨胀
 * （原 ast/compute 已 12 个类）；合并为「数据 + 运算种类」后，新增运算只需加一个常量、
 * 在前后端各加一个分支，类数量不再随运算增长。
 */
package ast;

public final class op {
    public static final int ADD = 0;
    public static final int SUB = 1;
    public static final int MUL = 2;
    public static final int DIV = 3;
    public static final int REM = 4;
    public static final int EQ = 5;
    public static final int NE = 6;
    public static final int LT = 7;
    public static final int LE = 8;
    public static final int GT = 9;
    public static final int GE = 10;
    public static final int NEG = 11;

    private op() {
    }
}