/*
 * 函数级代码生成入口。
 *
 * 做什么：把一个 ast.function 编译为 .text 段字节——建立栈帧、按序发射块内语句、
 *   以块的尾表达式作为返回值（EAX），最后拆除栈帧。
 * 提供什么功能：emit(function fn, arch out)。
 * 语句的机器码通过 arch 接口下达，与具体目标架构解耦。
 */
package backend;

import ast.block;
import ast.function;
import ast.letstmt;
import ast.stmt;

public final class codegen {
    public static void emit(function fn, arch out) {
        out.begin(fn.frameBytes);
        emitBlock(fn.body, out);
        out.end();
    }

    private static void emitBlock(block b, arch out) {
        for (stmt s : b.stmts) emitStmt(s, out);
        if (b.tail != null) eval.emit(b.tail, out);
    }

    private static void emitStmt(stmt s, arch out) {
        if (s instanceof letstmt l) {
            eval.emit(l.init, out);
            out.storeEax(l.offset);
        }
    }
}