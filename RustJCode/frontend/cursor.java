/*
 * token 游标。
 *
 * 做什么：封装语法分析器对 token 流的低层读写（读当前位置、前进、匹配、报错），
 *         使 parser 只专注文法规则本身。
 *
 * 提供什么功能：
 *   - cursor(List<token> tokens)：绑定 token 流，游标置于起始处。
 *   - next(kind, what)：按类型读取当前单元并前进，类型不符则带行号报错。
 *   - peek(text)：判断当前单元的原文是否等于给定串（不前进）。
 *   - advance()：跳过当前单元。
 *   - take()：取出当前单元并前进。
 *   - expect(text)：按原文匹配当前单元并前进，不符则报错。
 */
package frontend;

import error.rustjerror;
import java.util.List;

public final class cursor {
    private final List<token> tokens;
    private int at;

    public cursor(List<token> tokens) {
        this.tokens = tokens;
    }

    public token next(token.kind kind, String what) {
        token t = tokens.get(at);
        if (t.type != kind) throw new rustjerror(t.line, "期望" + what + "，实际是 \"" + t.text + "\"");
        at++;
        return t;
    }

    public boolean peek(String text) {
        return tokens.get(at).text.equals(text);
    }

    public void advance() {
        at++;
    }

    public token take() {
        return tokens.get(at++);
    }

    public void expect(String text) {
        token t = tokens.get(at);
        if (!t.text.equals(text)) throw new rustjerror(t.line, "期望 \"" + text + "\"，实际是 \"" + t.text + "\"");
        at++;
    }
}