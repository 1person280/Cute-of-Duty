/*
 * 表达式代码生成。
 *
 * 做什么：按节点类型递归发射表达式的求值指令，结果统一落在 EAX。
 * 提供什么功能：emit(expr e, arch out)——分派 intlit / ident / unop / binop；
 *   二元运算按 ast.op 种类选择指令序列（算术用栈机保序，比较用 cmp+setcc+movzx）。
 * 本类不含任何架构细节，全部通过 arch 接口下达，故可复用于任意后端。
 */
package backend;

import ast.binop;
import ast.expr;
import ast.ident;
import ast.intlit;
import ast.op;
import ast.unop;

public final class eval {
    public static void emit(expr e, arch out) {
        if (e instanceof intlit n) { out.movEaxImm(n.value); return; }
        if (e instanceof ident v) { out.loadEax(v.offset); return; }
        if (e instanceof unop u) { emit(u.operand, out); out.negEax(); return; }
        emitBinary((binop) e, out);
    }

    /* 栈机求值：左值入栈 → 右值算入 EAX → 取出左值 → 运算结果回 EAX。 */
    private static void emitBinary(binop b, arch out) {
        emit(b.left, out);
        out.pushEax();
        emit(b.right, out);
        switch (b.kind) {
            case op.ADD -> { out.popEcx(); out.addEaxEcx(); }
            case op.MUL -> { out.popEcx(); out.imulEaxEcx(); }
            case op.SUB -> { order(out); out.subEaxEcx(); }
            case op.DIV -> { order(out); out.cdq(); out.idivEcx(); }
            case op.REM -> { order(out); out.cdq(); out.idivEcx(); out.movEaxEdx(); }
            case op.EQ -> { eq(out); out.setE(); out.movzxEaxAl(); }
            case op.NE -> { eq(out); out.setNE(); out.movzxEaxAl(); }
            case op.LT -> { eq(out); out.setL(); out.movzxEaxAl(); }
            case op.LE -> { eq(out); out.setLE(); out.movzxEaxAl(); }
            case op.GT -> { eq(out); out.setG(); out.movzxEaxAl(); }
            case op.GE -> { eq(out); out.setGE(); out.movzxEaxAl(); }
            default -> { }
        }
    }

    /* ecx = 右 → eax = 左（保序，供不满足交换律的减法/除法使用）。 */
    private static void order(arch out) {
        out.movEcxEax();
        out.popEax();
    }

    /* 比较前置：取回左值到 EAX、右值到 ECX，再 cmp 置标志位。 */
    private static void eq(arch out) {
        order(out);
        out.cmpEaxEcx();
    }
}