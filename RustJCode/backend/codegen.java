/*
 * 函数级代码生成入口。
 *
 * 做什么：把一个 ast.function 编译为 .text 段字节——建立栈帧、按序发射块内语句、
 *   以块的尾表达式作为返回值（EAX），最后拆除栈帧；并负责 if/while 的标号布局。
 * 提供什么功能：emit(function fn, arch out)。
 * 语句的机器码通过 arch 接口下达，与具体目标架构解耦。
 */
package backend;

import ast.assignstmt;
import ast.block;
import ast.function;
import ast.ifstmt;
import ast.letstmt;
import ast.returnstmt;
import ast.stmt;
import ast.whilestmt;

public final class codegen {
    public static void emit(function fn, arch out) {
        out.begin(fn.frameBytes);
        int end = out.newLabel();
        emitBlock(fn.body, out, end);
        out.bind(end);
        out.end();
    }

    private static void emitBlock(block b, arch out, int end) {
        for (stmt s : b.stmts) emitStmt(s, out, end);
        if (b.tail != null) eval.emit(b.tail, out);
    }

    private static void emitStmt(stmt s, arch out, int end) {
        if (s instanceof letstmt l) { eval.emit(l.init, out); out.storeEax(l.offset); }
        else if (s instanceof assignstmt a) { eval.emit(a.value, out); out.storeEax(a.offset); }
        else if (s instanceof returnstmt r) emitReturn(r, out, end);
        else if (s instanceof ifstmt i) emitIf(i, out, end);
        else if (s instanceof whilestmt w) emitWhile(w, out, end);
    }

    /* return：算出返回值（可缺省）后无条件跳到函数尾声标号。 */
    private static void emitReturn(returnstmt r, arch out, int end) {
        if (r.value != null) eval.emit(r.value, out);
        out.jump(end);
    }

    /* if：条件为真走 then，为假走 else（缺省则跳过）；两路汇合于 done。 */
    private static void emitIf(ifstmt s, arch out, int end) {
        int alt = out.newLabel();
        int done = out.newLabel();
        eval.emit(s.cond, out);
        out.branchIfZero(alt);
        emitBlock(s.then, out, end);
        out.jump(done);
        out.bind(alt);
        if (s.els != null) emitBlock(s.els, out, end);
        out.bind(done);
    }

    /* while：循环头重估条件，为假跳出口；循环体尾部无条件跳回循环头。 */
    private static void emitWhile(whilestmt s, arch out, int end) {
        int top = out.newLabel();
        int done = out.newLabel();
        out.bind(top);
        eval.emit(s.cond, out);
        out.branchIfZero(done);
        emitBlock(s.body, out, end);
        out.jump(top);
        out.bind(done);
    }
}