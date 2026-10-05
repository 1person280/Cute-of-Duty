/*
 * 一个函数的语法树表示（纯数据）。
 *
 * 做什么：承载 `fn 名字() -> i32 <块>` 的整体结构。
 * 提供什么功能：
 *   - name：函数名，决定产物符号名与输出文件名；
 *   - body：函数体块（语句列表 + 尾表达式），尾表达式的值即返回值（EAX）；
 *   - frameBytes：栈帧字节数，由语法分析期按局部变量总数算出。
 */
package ast;

public final class function {
    public final String name;
    public final block body;
    public final int frameBytes;

    public function(String name, block body, int frameBytes) {
        this.name = name;
        this.body = body;
        this.frameBytes = frameBytes;
    }
}