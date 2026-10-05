/*
 * 语法分析器（函数级入口）。
 *
 * 做什么：解析函数签名，并把函数体块（语句 + 尾表达式）的解析委托给 frontend.stmts。
 *
 * 提供什么功能：
 *   - parser(List<token> tokens)：绑定 token 流。
 *   - parse()：产出 ast.function（函数名 + 函数体块 + 栈帧字节数）。
 */
package frontend;

import ast.block;
import ast.function;
import java.util.List;

public final class parser {
    private final cursor cur;

    public parser(List<token> tokens) {
        this.cur = new cursor(tokens);
    }

    public function parse() {
        cur.expect("fn");
        String name = cur.next(token.kind.IDENT, "函数名").text;
        cur.expect("(");
        cur.expect(")");
        cur.next(token.kind.ARROW, "->");
        cur.expect("i32");
        locals syms = new locals();
        block body = new stmts(cur).parse(syms);
        return new function(name, body, syms.size() * 4);
    }
}